use prometrics::{self, Registry};
use prometrics::metrics::{CounterBuilder, HistogramBuilder};

#[derive(Debug, Clone)]
pub struct MetricsBuilder {
    name: Option<String>,
    registry: Registry,
}
impl MetricsBuilder {
    pub fn new() -> Self {
        MetricsBuilder {
            name: None,
            registry: prometrics::default_registry(),
        }
    }
    pub fn name(&mut self, name: &str) {
        self.name = Some(name.to_owned());
    }
    pub fn registry(&mut self, registry: Registry) {
        self.registry = registry;
    }
    pub fn counter(&self, name: &str) -> CounterBuilder {
        let mut builder = CounterBuilder::new(name);
        builder.namespace("tasque").registry(self.registry.clone());
        if let Some(ref name) = self.name {
            builder.label("name", name);
        }
        builder
    }
    pub fn histogram(&self, name: &str) -> HistogramBuilder {
        let mut builder = HistogramBuilder::new(name);
        builder.namespace("tasque").registry(self.registry.clone());
        if let Some(ref name) = self.name {
            builder.label("name", name);
        }
        builder
    }
}
