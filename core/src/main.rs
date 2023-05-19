use design_tokens::DesignTokens;

fn main() {
    std::fs::write("tmp/testy.txt", format!("{:#?}", data)).unwrap();
    std::fs::write("tmp/testy.css", data.to_css()).unwrap();
}
