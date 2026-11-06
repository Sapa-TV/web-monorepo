use super::*;

fn row(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|s| s.to_string()).collect()
}

#[test]
fn spreadsheet_id_from_variants() {
    assert_eq!(
        parse_spreadsheet_id("1Tv9aAfZTM1oGV9Pk-ANSJz--UiR_IObe1GlvkZhLPdk").as_deref(),
        Some("1Tv9aAfZTM1oGV9Pk-ANSJz--UiR_IObe1GlvkZhLPdk")
    );
    assert_eq!(
        parse_spreadsheet_id(
            "https://docs.google.com/spreadsheets/d/ABC123/edit?pli=1&gid=0#gid=0"
        )
        .as_deref(),
        Some("ABC123")
    );
    assert_eq!(
        parse_spreadsheet_id(" https://docs.google.com/spreadsheets/d/XYZ/ ").as_deref(),
        Some("XYZ")
    );
    assert_eq!(parse_spreadsheet_id(""), None);
    assert_eq!(parse_spreadsheet_id("https://google.com/"), None);
}

#[test]
fn game_rows_skip_headers_and_parse_fields() {
    let values = vec![
        row(&["", "", "Пройденная", "Отмена", "", "", "легенда"]),
        row(&[
            "",
            "№",
            "ИГРА",
            "ЗАКАЗЧИК",
            "Стрим \\ прохождение",
            "Дата",
            "КОММЕНТАРИИ",
        ]),
        row(&[
            "",
            "1",
            "Game Alpha",
            "Аноним",
            "Прохождение",
            "22.10.2023",
            "атмосферная",
        ]),
        row(&[
            "",
            "18",
            "Game Beta",
            "user_five (рулетка)",
            "Стрим",
            "18.01.2024",
            "",
        ]),
        row(&["", "99", "Игра Гамма", "user_eight", "30ч (донат)", "", ""]),
        row(&[
            "",
            "101",
            "Game Delta",
            "user_nine",
            "стрим (баллы)",
            "---------",
            "",
        ]),
        row(&["", "102", "", "user_ten", "прохождение (донат)", "", ""]),
        row(&["", "", "", "", "", "", ""]),
    ];

    let games = parse_game_rows(&values);
    assert_eq!(games.len(), 5);

    assert_eq!(games[0].title.as_deref(), Some("Game Alpha"));
    assert_eq!(games[0].kind, GameOrderKind::Playthrough);
    assert_eq!(games[0].source, OrderSource::Other);
    assert_eq!(games[0].status, OrderStatus::Completed);
    assert_eq!(games[0].comment.as_deref(), Some("атмосферная"));

    assert_eq!(games[1].source, OrderSource::Other);
    assert_eq!(games[1].kind, GameOrderKind::Stream);

    assert_eq!(games[2].source, OrderSource::Donate);
    assert_eq!(games[2].status, OrderStatus::Pending);

    assert_eq!(games[3].source, OrderSource::Points);
    assert_eq!(games[3].status, OrderStatus::Pending);

    assert_eq!(games[4].title, None);
    assert_eq!(games[4].kind, GameOrderKind::Playthrough);
}

#[test]
fn movie_rows_parse_customer_and_title() {
    let values = vec![
        row(&["", "№", "Фильм", "ЗАКАЗЧИК", "Фильм\\Сериал"]),
        row(&["", "341", "", "user_six", "Фильм тест"]),
        row(&["", "423", "", "user_seven (user_three)", ""]),
        row(&["", "424", "", "", ""]),
    ];

    let movies = parse_movie_rows(&values);
    assert_eq!(movies.len(), 2);
    assert_eq!(movies[0].customer_name, "user_six");
    assert_eq!(movies[0].title.as_deref(), Some("Фильм тест"));
    assert_eq!(movies[0].kind, MovieKind::Movie);
    assert_eq!(movies[1].title, None);
}

#[test]
fn vip_rows_parse_kind_and_dates() {
    let values = vec![
        row(&["", "447", "VIP", "vip_user", "08.06.2026", "22.06.2026"]),
        row(&["", "457", "UnVIP", "unvip_user", "17.07.2026", "24.07.2026"]),
        row(&["", "480", "VIP", "", "13.01.1900", ""]),
        row(&["", "481", "VIP", "broken", "not-a-date", ""]),
    ];

    let vips = parse_vip_rows(&values);
    assert_eq!(vips.len(), 2);
    assert_eq!(vips[0].kind, VipKind::Vip);
    assert_eq!(
        vips[0].roulette_date,
        NaiveDate::from_ymd_opt(2026, 6, 8).unwrap()
    );
    assert_eq!(
        vips[0].end_date,
        Some(NaiveDate::from_ymd_opt(2026, 6, 22).unwrap())
    );
    assert_eq!(vips[1].kind, VipKind::Unvip);
}
