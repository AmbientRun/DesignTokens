use design_tokens::DesignTokens;

fn main() {
    let data = include_str!("../ambient-design/tokens.json");
    let data: DesignTokens = serde_json::from_str(data).unwrap();
    std::fs::write("tmp/testy.txt", format!("{:#?}", data)).unwrap();
    std::fs::write("tmp/testy.css", data.to_css()).unwrap();
}
