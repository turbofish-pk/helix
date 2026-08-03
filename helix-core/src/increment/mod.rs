mod date_time;
mod integer;

#[must_use]
pub fn integer(selected_text: &str, amount: i64) -> Option<String> {
    integer::increment(selected_text, amount)
}

#[must_use]
pub fn date_time(selected_text: &str, amount: i64) -> Option<String> {
    date_time::increment(selected_text, amount)
}
