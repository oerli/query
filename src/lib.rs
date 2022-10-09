use serde::{Deserialize, Serialize};

use worker::*;
use kv::*;

use rand::{distributions::Alphanumeric, Rng};
use chrono::Utc;

mod utils;


#[derive(Serialize, Deserialize, Debug)]
pub enum Type {
    Question{q: Question},
    Vote,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    session: String,
    lifetime: u64,
}

impl Session {
    pub async fn new(kv: &KvStore) -> Result<Session> {       
        loop {
            let key = rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>();
            if None == kv.get(&key).text().await? {
                return Ok(Session{
                    session: key,
                    lifetime: Utc::now().timestamp() as u64 + KV_TTL,
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Question {
    question: String,
    answers: Vec<Answer>,
    key: String,
}

impl Question {
    pub fn new(question: String, answers: Vec<Answer>) -> Question {
        return Question { question: question, answers: answers, key: rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>() }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Answer {
    answer: String,
    key: String,
}

impl Answer {
    pub fn new(answer: String) -> Answer {
        return Answer { answer: answer, key: rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>() }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Vote {
    pub vote: String,
    pub answer_key: Option<String>,
    pub question_key: Option<String>,
}

impl Vote {
    pub fn new(answer: String, vote: String) -> Vote {
        return Vote { vote: vote, answer_key: None, question_key: None }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Score {
    pub questions: Vec<Question>,
    pub votes: Vec<Vote>,
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

const GUI_URL: &str = "http://localhost:8080";

const KV_TTL: u64 = 2592000;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);    

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    let kv = KvStore::from_this(&env, "KV_QUERY")?;

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::with_data(kv);

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.

    router
        .options_async("/question", |_, _| async move {
            let cors = Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            return Response::empty()?.with_cors(&cors);
        })
        // create a question and receive the session id
        .post_async("/question", |mut req, ctx| async move {
            let cors = &Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            let kv = &ctx.data;

            let session = Session::new(kv).await?;

            let questions: Vec<Question> = req.json().await?;
            kv.put(&session.session, questions)?.expiration_ttl(KV_TTL).execute().await?;
            
            return Response::from_json(&session)?.with_cors(&cors);
        })
        // get the questions from the id
        .get_async("/question/:field", |_, ctx| async move {
            let cors = &Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            let kv = &ctx.data;
            
            if let Some(session) = ctx.param("field") {
                let question: Option<Vec<Question>> = kv.get(&session).json().await?;
                match question {
                    Some(q) => {
                        return Response::from_json(&q)?.with_cors(&cors);
                    },
                    None => {
                        return Response::error("Not Acceptable", 406)?.with_cors(&cors);
                    }
                }
            }
            return Response::error("Not Found", 401)?.with_cors(&cors);
        })
        .options_async("/vote/:field", |_, _| async move {
            let cors = Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            return Response::empty()?.with_cors(&cors);
        })
        // vote for a question
        .post_async("/vote/:field", |mut req, ctx| async move {
            let cors = &Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            let kv = &ctx.data;
            
            //TODO: update votes for each question? or use question id's?
            let votes: Vec<Vote> = req.json().await?;
            console_debug!("{:?}", votes);
            if let Some(question_session) = ctx.param("field") {
                let answer_session = rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>();
                let session = Session{session: format!("{}:{}", question_session, answer_session), lifetime: 0};
                
                kv.put(&session.session, votes)?.expiration_ttl(KV_TTL).execute().await?;
                return Response::from_json(&session)?.with_cors(&cors);
            } else {
                Response::error("Not Acceptable", 406)?.with_cors(&cors)
            }
        })
        .options_async("/result", |_, _| async move {
            let cors = Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            return Response::empty()?.with_cors(&cors);
        })
        // display the results of the votes
        .get_async("/result/:field", |_, ctx| async move {
            let cors = &Cors::with_origins(Cors::new(), vec![GUI_URL]).with_methods(vec![Method::Get, Method::Options, Method::Post]).with_allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"]);
            let kv = &ctx.data;
            
            if let Some(session) = ctx.param("field") {
                match kv.get(&session).json().await? {
                    Some(questions) => {
                        let mut votes: Vec<Vote> = Vec::new();
        
                        let list = kv.list().prefix(format!("{}:", session)).execute().await?;
                                
                        for key in list.keys {
                            match &mut kv.get(&key.name).json().await? {
                                Some(v) => {votes.append(v);},
                                None => {},
                            }
                        }
                        let score = Score{questions, votes};
                        return Response::from_json(&score)?.with_cors(&cors);
                    },
                    None => ()
                }
            }
            return Response::error("Not Found", 401)?.with_cors(&cors);
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}