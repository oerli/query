use serde::{Serialize, Deserialize};
use handlebars::Handlebars;
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct Question {
    question: String,
    answer_1: String,
    answer_2: String,
    answer_3: String,
    answer_4: String,
    //dirty
    pub session: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Vote {
    pub vote: String,
}

impl Question {
    pub fn new() -> Question {
        return Question {
            question: "Question".to_owned(),
            answer_1: "Answer".to_owned(),
            answer_2: "".to_owned(),
            answer_3: "".to_owned(),
            answer_4: "".to_owned(),
            session: Some("".to_owned()),
        }
    }
    pub fn generate() {
        let mut reg = Handlebars::new();
        println!("{}", reg.render_template("Hello {{name}}", &json!({"name": "foo"})).unwrap());
    }
}
