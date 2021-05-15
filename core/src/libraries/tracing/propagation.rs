use super::global_tracer;
use opentelemetry::{
    global::{self, BoxedSpan},
    trace::Tracer,
    Context,
};
use std::collections::HashMap;

pub struct StringPropagator;

impl StringPropagator {
    pub fn serialize(context: &Context) -> Result<String, serde_json::Error> {
        let mut map = HashMap::new();

        global::get_text_map_propagator(|propagator| propagator.inject_context(context, &mut map));

        serde_json::to_string(&map)
    }

    pub fn deserialize(serialized_context: &str, span_name: &str) -> BoxedSpan {
        if let Ok(deserialized) =
            serde_json::from_str::<HashMap<String, String>>(serialized_context)
        {
            let parent_cx =
                global::get_text_map_propagator(|propagator| propagator.extract(&deserialized));

            global_tracer().start_with_context(span_name.to_owned(), parent_cx)
        } else {
            global_tracer().start(span_name.to_owned())
        }
    }
}
