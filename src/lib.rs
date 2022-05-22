use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use worker::*;
use kv::*;

use rand::{distributions::Alphanumeric, Rng};

mod utils;


#[derive(Serialize, Deserialize, Debug)]
pub enum Type {
    Question{q: Question},
    Vote,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    session: String,
}

impl Session {
    pub async fn new(kv: &KvStore) -> Result<Session> {       
        loop {
            let key = rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>();
            if None == kv.get(&key).text().await? {
                return Ok(Session{
                    session: key
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Question {
    question: String,
    answers: Vec<Answer>,
}

impl Question {
    pub fn new(question: String, answers: Vec<Answer>) -> Question {
        return Question { question: question, answers: answers }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Answer {
    answer: String,
    id: String,
}

impl Answer {
    pub fn new(answer: String) -> Answer {
        return Answer { answer: answer, id: rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>() }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Vote {
    answer: String,
    vote: String
}

impl Vote {
    pub fn new(answer: String, vote: String) -> Vote {
        return Vote { answer: answer, vote: vote }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Count {
    answer: String,
    count: u16,
}

impl Count {
    pub fn new(answer: String, count: u16) -> Count {
        return Count { answer: answer, count: count }
    }
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);    

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    let kv = KvStore::from_this(&env, "KV_QUESTIONS")?;

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::with_data(kv);

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        // create a question and receive the session id
        .post_async("/q", |mut req, ctx| async move {
            let kv = &ctx.data;

            let session = Session::new(kv).await?;

            let question: Question = req.json().await?;
            kv.put(&session.session, question)?.execute().await?;

            return Response::from_json(&session);
        })
        // get the questions from the id
        .get_async("/q/:field", |_, ctx| async move {
            let kv = &ctx.data;
            
            if let Some(session) = ctx.param("field") {
                let question: Option<Question> = kv.get(&session).json().await?;
                match question {
                    Some(q) => {
                        return Response::from_json(&q);
                    },
                    None => {
                        return Response::error("Not Acceptable", 406);
                    }
                }
            }
            return Response::error("Not Found", 401);
        })
        // vote for a question
        .post_async("/v/:field", |mut req, ctx| async move {
            let kv = &ctx.data;
            
            let votes: Vec<Vote> = req.json().await?;
            if let Some(question) = ctx.param("field") {
                let session = rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>();
                kv.put(&format!("{}:{}", question, session), votes)?.execute().await?;
            }
            Response::error("Not Acceptable", 406)
        })
        // display the results of the votes
        .get_async("/r/:field", |_, ctx| async move {
            let kv = &ctx.data;
            
            if let Some(question) = ctx.param("field") {
                let list = kv.list().prefix(format!("{}:", question)).execute().await?;
                
                let mut results: HashMap<String, HashMap<String, u16>> = HashMap::new();

                for key in list.keys {
                    let votes: Option<Vec<Vote>> = kv.get(&key.name).json().await?;
                    
                    match votes {
                        Some(v) => {
                            for vote in v {
                                match results.get_mut(&vote.answer) {
                                    Some(c) => {
                                        match c.get(&vote.vote) {
                                            Some(x) => {
                                                let i = x + 1;
                                                c.insert(vote.vote, i);
                                            },
                                            None => {
                                                c.insert(vote.vote, 1);
                                            }
                                        }
                                        
                                    },
                                    None => {
                                        let mut count: HashMap<String, u16> = HashMap::new();
                                        count.insert(vote.vote, 1);

                                        results.insert(vote.answer, count);
                                    }
                                }
                            }
                        },
                        None => {}
                    }
                }
                
                return Response::from_json(&results);
                
            }
            return Response::error("Not Found", 401);
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}