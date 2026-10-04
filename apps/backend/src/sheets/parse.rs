use chrono::NaiveDate;

use crate::orders::game::{GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::movie::{MovieKind, NewMovieOrder};
use crate::orders::status::OrderStatus;
use crate::orders::vip::{NewVipRecord, VipKind};

fn cell(row: &[String], idx: usize) -> &str {
    row.get(idx).map(|s| s.trim()).unwrap_or("")
}

fn opt(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_data_row(row: &[String]) -> bool {
    cell(row, 1).parse::<u32>().is_ok()
}

fn parse_ru_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%d.%m.%Y").ok()
}

/// Accepts a raw spreadsheet id or any Google Sheets URL containing /d/{id}.
pub fn parse_spreadsheet_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed.contains('/') {
        return Some(trimmed.to_string());
    }
    let mut parts = trimmed.split('/');
    while let Some(part) = parts.next() {
        if part == "d" {
            if let Some(id) = parts.next()
                && !id.is_empty()
            {
                return Some(id.to_string());
            }
            return None;
        }
    }
    None
}

/// Games tab: [_, №, title, customer, kind_text, completed_at, comment].
pub fn parse_game_rows(values: &[Vec<String>]) -> Vec<NewGameOrder> {
    values
        .iter()
        .filter(|row| is_data_row(row))
        .filter_map(|row| {
            let customer = cell(row, 3);
            if customer.is_empty() {
                return None;
            }
            let kind_text = cell(row, 4).to_lowercase();
            let kind = if kind_text.contains("прохожд") {
                GameOrderKind::Playthrough
            } else {
                GameOrderKind::Stream
            };
            let source = if kind_text.contains("рулетк") {
                OrderSource::Roulette
            } else if kind_text.contains("баллы") {
                OrderSource::Points
            } else if kind_text.contains("донат") {
                OrderSource::Donate
            } else {
                OrderSource::Other
            };
            let completed_at = parse_ru_date(cell(row, 5));
            let status = if completed_at.is_some() {
                OrderStatus::Completed
            } else {
                OrderStatus::Pending
            };
            Some(NewGameOrder::new(
                opt(cell(row, 2)),
                customer.to_string(),
                None,
                kind,
                source,
                status,
                completed_at,
                opt(cell(row, 6)),
            ))
        })
        .collect()
}

/// Movies tab: [_, №, _, customer, title, ...].
pub fn parse_movie_rows(values: &[Vec<String>]) -> Vec<NewMovieOrder> {
    values
        .iter()
        .filter(|row| is_data_row(row))
        .filter_map(|row| {
            let customer = cell(row, 3);
            if customer.is_empty() {
                return None;
            }
            Some(NewMovieOrder::new(
                opt(cell(row, 4)),
                customer.to_string(),
                None,
                MovieKind::Movie,
                OrderSource::Other,
                OrderStatus::Pending,
                None,
            ))
        })
        .collect()
}

/// VIP tab: [_, №, vip/unvip, customer, roulette_date, end_date].
pub fn parse_vip_rows(values: &[Vec<String>]) -> Vec<NewVipRecord> {
    values
        .iter()
        .filter(|row| is_data_row(row))
        .filter_map(|row| {
            let customer = cell(row, 3);
            if customer.is_empty() {
                return None;
            }
            let kind = if cell(row, 2).to_lowercase().contains("unvip") {
                VipKind::Unvip
            } else {
                VipKind::Vip
            };
            let roulette_date = parse_ru_date(cell(row, 4))?;
            Some(NewVipRecord::new(
                customer.to_string(),
                None,
                kind,
                roulette_date,
                parse_ru_date(cell(row, 5)),
                None,
            ))
        })
        .collect()
}

#[cfg(test)]
#[path = "parse.test.rs"]
mod tests;
