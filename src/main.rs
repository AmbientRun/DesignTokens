use design_tokens::{DesignTokens, TokenOrGroup};

fn main() {
    let data = include_str!("../ambient-design/tokens.json");
    let mut data: DesignTokens = serde_json::from_str(data).unwrap();
    if let TokenOrGroup::Group(group) = &mut data.global {
        group.retain(|k, v| k == "Brand" || k == "Set");
    }
    std::fs::write("tmp/testy.txt", format!("{:#?}", data));
    std::fs::write("tmp/testy.css", data.to_css());
}
