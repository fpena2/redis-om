use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
struct Record {
    items: Vec<String>,
    #[redis(rename = "items.0")]
    first_item_label: String,
}

fn main() {}
