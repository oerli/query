use std::collections::HashMap;

use serde_json::json;
use worker::*;
use kv::*;

use rand::{distributions::Alphanumeric, Rng};
use handlebars::Handlebars;

mod utils;
mod question;

use question::{Vote, Question};

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
        .get_async("/", |_, ctx| async move {
            
            let kv = &ctx.data;

            loop {
                let key = rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>(); 
            
                if None == kv.get(&key).text().await? {
                    // let url = Url::parse(format!("http://localhost:8787/q/{}", key).as_str())?;
                    let mut headers = Headers::new();
                    headers.append("location", &format!("/q/{}", &key));
                    // return Response::empty().unwrap_or(Response::error("Error creating Response", 503)).with_status(302).with_headers(headers);
                    match Response::empty() {
                        Ok(r) => {
                            return Ok(r.with_status(302).with_headers(headers));
                        },
                        Err(_) => {
                            return Response::error("error creating response", 503);
                        }
                    }
                }
            }
            
            // return Response::ok(s);
            // return Response::ok("Hello from Workers!");
        })
        .get_async("/a/:field", |_, ctx| async move {
            
            let kv = &ctx.data;
            if let Some(name) = ctx.param("field") {
                let list = kv.list().prefix(name[0..5].to_string()).execute().await?;

                let mut result: Vec<Vote> = Vec::new();
                for key in list.keys {
                    kv.get(&key.name).text().await;
                    match kv.get(&key.name).text().await? {
                        Some(v) => {
                            match serde_json::from_str(&v) {
                                Ok(v) => {
                                    result.insert(result.len(), v);
                                    
                                },
                                _ => {}
                            }
                        },
                        None => {}
                    }
                }

                let mut result_count: HashMap<String, u8> = HashMap::new();

                for r in result {
                    match result_count.get(&r.vote) {
                        Some(v) => {
                            result_count.insert(r.vote, v+1);
                        },
                        None => {
                            result_count.insert(r.vote, 1);
                        }
                    }
                }

                
                // return Response::ok(format!("{:?}", kv.list().prefix(name[0..5].to_string()).execute().await?));
                return Response::ok(format!("{:?}", result_count));
            }
            
            return Response::error("Bad Request", 400)

        })
        .post_async("/q/:field", |mut req, ctx| async move {
            let kv = &ctx.data;
            if let Some(name) = ctx.param("field") {
                let values: Question = req.json().await?;
                match kv.put(name, values) {
                    Ok(builder) => {
                        builder.execute().await?;
                        return Response::ok("Success :)");
                    }
                    Err(e) => {return Response::error(format!("Something went wrong: {}", e), 500);}
                }
                

                // match req.json().await {
                //     Ok(value) => {return Response::ok(format!("Value: {:?}", value));},
                //     Err(e) => {return Response::error(format!("Something went wrong: {}", e), 500);}
                // }
                
                
                // match form.get(name) {
                // match form.get("fname") {
                //     Some(FormEntry::Field(value)) => {
                //         // match kv.put(name, &json!({ name: value })) {
                //         match kv.put(name, value) {
                //             Ok(builder) => {
                //                 builder.execute().await?;
                //                 return Response::ok("Success :)");
                                
                //                 // return Response::from_json(&json!({ name: value }));
                //             },
                //             Err(e) => {
                //                 return Response::error(format!("Error: {}", e), 500);
                //             },
                //         }
                        
                //     }
                //     Some(FormEntry::File(_)) => {
                //         return Response::error("`field` param in form shouldn't be a File", 422);
                //     }
                //     None => return Response::error("Bad Request", 400),
                // }
            }

            Response::error("Bad Request", 400)
        }).post_async("/a/:field", |mut req, ctx| async move {
            let kv = &ctx.data;
            if let Some(name) = ctx.param("field") {
                let values: Vote = req.json().await?;
                match kv.put(name, values) {
                    Ok(builder) => {
                        builder.execute().await?;
                        // let url = Url::parse(format!("http://localhost:8787/a/{}", name).as_str())?;
                        // return Response::redirect(url);
                        return Response::ok("Voted :)");
                    }
                    Err(e) => {return Response::error(format!("Voting went wrong: {}", e), 500);}
                }  
            }

            Response::error("Bad Request", 400)
        })
        .get_async("/q/:field", |_, ctx | async move {

            let kv = &ctx.data;
            let mut reg = Handlebars::new();

            if let Some(name) = ctx.param("field") {
                let (value, metadata) = kv.get(name).text_with_metadata::<Vec<usize>>().await?;
                match value {
                    Some(v) => {
                        let mut question: Question = serde_json::from_str(&v)?;
                        //dirty fix
                        question.session = Some(format!("{}{}", name.to_string(), rand::thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect::<String>())); 
                        // let question = json!(&v);
                        return Response::from_html(
                            reg.render_template(r#"
                            <body>
                                <p>{{question}}:</p>

                                <form id="answer_form" method="post">    
                                    <input type="radio" id="answer_1" name="vote" value="1">
                                    <label for="answer_1">{{answer_1}}</label><br>
                                    <input type="radio" id="answer_2" name="vote" value="2">
                                    <label for="answer_2">{{answer_2}}</label><br>
                                    <input type="radio" id="answer_3" name="vote" value="3">
                                    <label for="answer_3">{{answer_3}}</label><br>
                                    <input type="radio" id="answer_4" name="vote" value="4">
                                    <label for="answer_4">{{answer_4}}</label><br>
                                    <input type="submit" value="Vote"><br>

                                    Answers: <a href="/a/{{session}}">/a/{{session}}</a>
                                </form> 
                            </body>
                            <script>
                                function handleSubmit(event) {
                                    event.preventDefault();
                                    const formData = new FormData(event.target);
                                    const data = {};
                                    formData.forEach((value, key) => (data[key] = value));
                                    
                                    console.log(data);

                                    const request = new XMLHttpRequest();
                                    request.open("POST", "/a/{{session}}");
                                    request.send(JSON.stringify(data));
                                }
                                const form = document.querySelector('#answer_form');
                                form.addEventListener('submit', handleSubmit);
                            </script>
                        "#, &question).unwrap());
                    },
                    None => {
                        return Response::from_html(
                            reg.render_template(r#"
                                <body>
                                    <form id="register_form" method="post">
                                        <label for="question">Question:</label><br>
                                        <input type="text" id="question" name="question" value="Question"><br>
                                        <label for="answer_1">Answer 1:</label><br>
                                        <input type="text" id="answer_1" name="answer_1" value="Answer"><br>
                                        <label for="answer_2">Answer 2:</label><br>
                                        <input type="text" id="answer_2" name="answer_2" value=""><br>
                                        <label for="answer_3">Answer 3:</label><br>
                                        <input type="text" id="answer_3" name="answer_3" value=""><br>
                                        <label for="answer_4">Answer 4:</label><br>
                                        <input type="text" id="answer_4" name="answer_4" value=""><br>
                                        <input type="submit" value="Submit"><br>
                                    </form>
                                    Link to Vote: <a href="/q/{{session}}">/q/{{session}}</a>
                                </body>
                                <script>
                                    function handleSubmit(event) {
                                        event.preventDefault();
                                        const formData = new FormData(event.target);
                                        const data = {};
                                        formData.forEach((value, key) => (data[key] = value));
                                        
                                        console.log(data);

                                        const request = new XMLHttpRequest();
                                        request.open("POST", "/q/{{session}}");
                                        request.send(JSON.stringify(data));
                                    }
                                    const form = document.querySelector('#register_form');
                                    form.addEventListener('submit', handleSubmit);
                                </script>
                            "#, &json!({"session": name})).unwrap());
                                }
                            }
                        }

                        return Response::error("error", 501);

                        // return Response::from_html(r#"
                        //     <body>
                        //         <form id="register_form" method="post">
                        //             <label for="fname">First name:</label><br>
                        //             <input type="text" id="fname" name="fname" value="Roland"><br>
                        //             <label for="lname">Last name:</label><br>
                        //             <input type="text" id="lname" name="lname" value="Mueller"><br>
                        //             <input type="submit" value="Submit">
                        //         </form> 
                        //     </body>
                        //     <script>
                        //         function handleSubmit(event) {
                        //             event.preventDefault();
                        //             const formData = new FormData(event.target);
                        //             const data = {};
                        //             formData.forEach((value, key) => (data[key] = value));
                                    
                        //             console.log(data);

                        //             const request = new XMLHttpRequest();
                        //             request.open("POST", "/q/9Gw5JB");
                        //             request.send(JSON.stringify(data));
                        //         }
                        //         const form = document.querySelector('#register_form');
                        //         form.addEventListener('submit', handleSubmit);
                        //     </script>
                        // "#);
                        // match kv.put(name, json!(question::Question::new()).to_string()) {
                        //     Ok(builder) => {
                        //         return Response::from_html(r#"
                        //             <body>
                        //                 <form id="register_form" method="post">
                        //                     <label for="fname">First name:</label><br>
                        //                     <input type="radio" id="fname" name="name" value="Roland"><br>
                        //                     <label for="lname">Last name:</label><br>
                        //                     <input type="radio" id="lname" name="name" value="Mueller"><br>
                        //                     <input type="submit" value="Submit">
                        //                 </form> 
                        //             </body>
                        //         "#);
                        //     },
                        //     Err(e) => {
                        //         return Response::error(format!("error: {}", e), 500);
                        //     },
                        // }
                        


            // let (value, metadata) = kv.get("text").text_with_metadata::<Vec<usize>>().await?;
            // match value {
            //     Some(v) => {
            //         let mut reg = Handlebars::new();
            //         return Response::from_html(
            //             reg.render_template(r#"
            //                 <body>
            //                     <form id="register_form" method="post">
            //                         <label for="fname">First name:</label><br>
            //                         <input type="text" id="fname" name="fname" value="Roland"><br>
            //                         <label for="lname">Last name:</label><br>
            //                         <input type="text" id="lname" name="lname" value="Mueller"><br>
            //                         <input type="submit" value="Submit">
            //                     </form> 
            //                 </body>
            //                 <script>
            //                     function handleSubmit(event) {
            //                         event.preventDefault();
            //                         const formData = new FormData(event.target);
            //                         const data = {};
            //                         formData.forEach((value, key) => (data[key] = value));
                                    
            //                         console.log(data);

            //                         const request = new XMLHttpRequest();
            //                         request.open("POST", "/q/{{session}}");
            //                         request.send(JSON.stringify(data));
            //                     }
            //                     const form = document.querySelector('#register_form');
            //                     form.addEventListener('submit', handleSubmit);
            //                 </script>
            //             "#, &json!({"session": name})).unwrap());
            //     },
            //     None => {
            //         match kv.put("text", json!(question::Question::new()).to_string()) {
            //             Ok(builder) => {
            //                 builder.execute().await?;
            //                 return Response::ok("Data written");
            //             },
            //             Err(e) => {
            //                 return Response::error(format!("error: {}", e), 500);
            //             },
            //         }
                    
            //     }
            // }

            // let version = value.ok_or("empty value".to_owned())?;
            // Response::ok(version)
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}





// return Response::from_html(r#"
//                                     <head>
//                                         <script src="https://ajax.googleapis.com/ajax/libs/jquery/3.3.1/jquery.min.js"></script>
//                                         <script>
//                                             const serialize_form = form => JSON.stringify(
//                                                 Array.from(new FormData(form).entries())
//                                                     .reduce((m, [ key, value ]) => Object.assign(m, { [key]: value }), {})
//                                             );
                                          
//                                             $('#register_form').on('submit', function(event) {
//                                                 event.preventDefault();
//                                                 const json = serialize_form(this);
//                                                 console.log(json);
//                                                 /*$.ajax({
//                                                 type: 'POST',
//                                                 url: 'http://localhost:8787/s/test',
//                                                 dataType: 'json',
//                                                 data: json,
//                                                 contentType: 'application/json',
//                                                 success: function(data) {
//                                                     alert(data)
//                                                 }
//                                                 });*/
//                                             });
//                                         </script>
//                                     </head>
//                                     <body>
//                                         <form id="register_form" method="post">
//                                             <label for="fname">First name:</label><br>
//                                             <input type="radio" id="fname" name="fname" value="Roland"><br>
//                                             <label for="lname">Last name:</label><br>
//                                             <input type="radio" id="lname" name="lname" value="Mueller"><br>
//                                             <input type="submit" value="Submit">
//                                         </form> 
//                                     </body>
//                                 "#);