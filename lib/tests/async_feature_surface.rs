// These signatures are compile-time feature assertions and are not executed.
#![allow(dead_code)]

use redis_om::HashModel;
use std::future::Future;

#[derive(HashModel)]
struct Record {
    id: String,
}

fn assert_redis_future<F: Future<Output = redis_om::RedisResult<()>>>(_: F) {}

#[cfg(feature = "tokio-native-tls-comp")]
fn tokio_tls_async_surface<C>(record: &mut Record, connection: &mut C)
where
    C: redis_om::redis::aio::ConnectionLike + Send,
{
    assert_redis_future(record.save(connection));
}

#[cfg(feature = "smol-native-tls-comp")]
fn smol_tls_async_surface<C>(record: &mut Record, connection: &mut C)
where
    C: redis_om::redis::aio::ConnectionLike + Send,
{
    assert_redis_future(record.save(connection));
}
