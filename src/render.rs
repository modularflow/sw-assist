use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("failed to serialize json: {}", e),
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ErrorOut<'a> {
    pub code: &'a str,
    pub message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<&'a str>,
}

pub fn print_json_error(code: &str, message: &str, hint: Option<&str>) {
    let err = ErrorOut { code, message, hint };
    print_json(&err);
}

#[derive(Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct Feedback {
    pub correctness: Vec<String>,
    pub style: Vec<String>,
    pub security: Vec<String>,
    pub tests: Vec<String>,
    pub suggestions: Vec<String>,
}

pub fn render_review_text(feedback: &Feedback) {
    println!("Correctness:");
    for item in &feedback.correctness { println!("- {}", item); }
    println!("\nStyle:");
    for item in &feedback.style { println!("- {}", item); }
    println!("\nSecurity:");
    for item in &feedback.security { println!("- {}", item); }
    println!("\nTests:");
    for item in &feedback.tests { println!("- {}", item); }
    println!("\nSuggestions:");
    for item in &feedback.suggestions { println!("- {}", item); }
}

