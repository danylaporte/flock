pub trait EntityBy<K, V> {
    fn entity_by(&self, key: K) -> Option<&V>;
}
