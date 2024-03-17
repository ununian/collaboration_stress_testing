use rand::Rng;
use yrs::{block::ClientID, Options};

pub fn with_uuid_option(uuid: &String) -> Options {
    let mut rng = rand::thread_rng();
    let client_id: u32 = rng.gen();
    let uuid = uuid.as_str().into();
    Options::with_guid_and_client_id(uuid, client_id as ClientID)
}

pub fn to_hex_string(data: &Vec<u8>) -> String {
    let hex_groups = data
        .chunks(2) // 每2个字节作为一组
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    // 每8组（总共16个字节）换行
    hex_groups
        .chunks(8) // 每8个组为一行
        .map(|line| line.join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}
