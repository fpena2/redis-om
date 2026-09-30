use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
struct Record {
    #[redis(alias = "items.0")]
    items: Vec<String>,
}

fn main() {}
