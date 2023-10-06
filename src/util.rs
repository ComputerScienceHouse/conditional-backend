pub fn split_usernames(
    usernames: Vec<String>,
) -> Result<(Vec<u32>, Vec<String>), std::num::ParseIntError> {
    // let chom = usernames.into_iter()
    let (frosh, members): (Vec<String>, Vec<String>) =
        usernames.into_iter().partition(|username| {
            if let Some(c) = username.chars().next() {
                c.is_numeric()
            } else {
                false
            }
        });
    let frosh: Result<Vec<u32>, _> = frosh.iter().map(|user| user.parse()).collect();
    match frosh {
        Ok(frosh) => Ok((frosh, members)),
        Err(e) => Err(e),
    }
}
