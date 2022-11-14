pub fn fix_input_url(input_url: &str) -> String {
    let mut url = String::new();
    if !&input_url.ends_with('/') {
        let format = format!("{}/", input_url);
        url.push_str(&format);
        return url;
    }

    input_url.to_string()
}
