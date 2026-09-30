use redis_om::RedisTransportValue;

#[derive(RedisTransportValue)]
enum State {
    #[redis(skip_serializing)]
    Skipped,
    Visible,
}

fn main() {}
