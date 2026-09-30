use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
struct Record {
    items: Vec<String>,
    #[redis(rename = "items")]
    items_label: String,
}

fn main() {}
