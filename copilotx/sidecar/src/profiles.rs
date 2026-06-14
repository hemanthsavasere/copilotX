use std::collections::HashMap;

pub fn get_system_prompt(profile: &str) -> Option<String> {
    let profiles: HashMap<&str, &str> = HashMap::from([
        ("interview", "You are an expert interview assistant. When shown a screenshot of a coding problem, MCQ, or technical question, provide a concise, correct answer. For coding problems, give working code with brief explanation. For MCQs, give the answer with one-line reasoning."),
        ("sales", "You are a sales assistant. Help respond to objections and suggest talking points."),
        ("meeting", "You are a meeting assistant. Summarize discussions and suggest action items."),
        ("presentation", "You are a presentation assistant. Help with talking points and Q&A responses."),
        ("negotiation", "You are a negotiation assistant. Suggest strategies and counterarguments."),
    ]);
    profiles.get(profile).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interview_profile() {
        let prompt = get_system_prompt("interview").unwrap();
        assert!(prompt.contains("interview assistant"));
    }

    #[test]
    fn test_sales_profile() {
        let prompt = get_system_prompt("sales").unwrap();
        assert!(prompt.contains("sales assistant"));
    }

    #[test]
    fn test_meeting_profile() {
        let prompt = get_system_prompt("meeting").unwrap();
        assert!(prompt.contains("meeting assistant"));
    }

    #[test]
    fn test_unknown_profile_returns_none() {
        assert!(get_system_prompt("unknown").is_none());
    }

    #[test]
    fn test_empty_profile_returns_none() {
        assert!(get_system_prompt("").is_none());
    }
}
