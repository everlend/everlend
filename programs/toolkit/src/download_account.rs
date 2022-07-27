use reqwest::header::{HeaderMap, CONTENT_TYPE};
use serde_json::Value;
use solana_program::pubkey::Pubkey;
use std::fs::File;
use std::io::Write;

pub async fn download_account(pubkey: &Pubkey, mm_name: &str, account_name: &str) {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    let res = client
        .post("https://api.devnet.solana.com")
        .headers(headers)
        .body(format!(
            "
        {{
            \"jsonrpc\": \"2.0\",
            \"id\": 1,
            \"method\": \"getAccountInfo\",
            \"params\": [
                \"{}\",
                {{
                \"encoding\": \"base64\"
                }}
            ]
        }}
        ",
            pubkey.to_string()
        ))
        .send()
        .await
        .expect("failed to get response")
        .text()
        .await
        .expect("failed to get payload");
    let json: Value = serde_json::from_str(&res).unwrap();
    let data = &json["result"]["value"]["data"][0];
    let string = data.as_str().unwrap();
    let bytes = base64::decode(string).unwrap();
    let mut file = File::create(format!(
        "../tests/tests/fixtures/{}/{}.bin",
        mm_name, account_name
    ))
    .unwrap();
    file.write_all(bytes.as_slice()).unwrap();
    println!("{} {}", account_name, pubkey.to_string());
}
