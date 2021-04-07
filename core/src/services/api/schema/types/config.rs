use super::super::GqlContext;
use crate::libraries::helpers::Timeout;
use juniper::graphql_object;

pub struct Timeouts;

impl Timeouts {
    pub fn new() -> Self {
        Self {}
    }
}

#[graphql_object(context = GqlContext)]
impl Timeouts {
    pub async fn queue(context: &GqlContext) -> i32 {
        Timeout::Queue.get(&mut *context.redis.lock().await).await as i32
    }

    pub async fn scheduling(context: &GqlContext) -> i32 {
        Timeout::Scheduling
            .get(&mut *context.redis.lock().await)
            .await as i32
    }

    pub async fn nodeStartup(context: &GqlContext) -> i32 {
        Timeout::NodeStartup
            .get(&mut *context.redis.lock().await)
            .await as i32
    }

    pub async fn driverStartup(context: &GqlContext) -> i32 {
        Timeout::DriverStartup
            .get(&mut *context.redis.lock().await)
            .await as i32
    }

    pub async fn sessionTermination(context: &GqlContext) -> i32 {
        Timeout::SessionTermination
            .get(&mut *context.redis.lock().await)
            .await as i32
    }

    pub async fn slotReclaimInterval(context: &GqlContext) -> i32 {
        Timeout::SlotReclaimInterval
            .get(&mut *context.redis.lock().await)
            .await as i32
    }
}
