//! The single source of truth for "what timezone does the user think in".
//!
//! Three zones used to be in play independently — the Postgres session zone
//! (never set, so whatever the server defaulted to), the UTC we serialized
//! `timestamptz` into, and the browser's local zone that rendered it. Nothing
//! made them agree, so a grid cell, its tooltip, the CSV export and a
//! `DATE_TRUNC` day bucket could each disagree by the UTC offset.
//!
//! Now one configured zone (`[display] timezone`, default UTC) is resolved once
//! at startup and applied in every layer:
//!
//! 1. the Postgres session (`SET TimeZone`), so all SQL-side date math —
//!    `DATE_TRUNC`, `EXTRACT`, `AGE`, and text→`timestamptz` filter casts —
//!    is evaluated against it;
//! 2. Rust serialization, so the wire value carries that zone's real offset
//!    instead of a hardcoded `+00:00`;
//! 3. the frontend, which formats in it rather than the viewer's local zone.

use std::sync::OnceLock;

use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;

/// Resolved once during startup. Read through [`display_timezone`], which falls
/// back to UTC so unit tests and setup mode behave predictably without it set.
static DISPLAY_TIMEZONE: OnceLock<Tz> = OnceLock::new();

/// Parse an IANA timezone name (e.g. `Asia/Singapore`).
///
/// Rejects anything chrono-tz does not recognise, which also keeps the value
/// safe to hand to Postgres — but callers must still bind it as a parameter
/// rather than interpolate, see [`session_timezone_sql`].
pub fn parse(name: &str) -> anyhow::Result<Tz> {
    name.parse::<Tz>()
        .map_err(|_| anyhow::anyhow!("Unknown timezone {name:?} — expected an IANA name such as \"Asia/Singapore\" or \"UTC\""))
}

/// Install the process-wide display timezone. Called once from startup, after
/// config validation has already proven the name parses.
///
/// Returns the zone actually in effect: a second call is ignored (the first
/// wins) rather than panicking, so a setup-mode reconfigure cannot abort the
/// server. That case is logged by the caller.
pub fn set_display_timezone(tz: Tz) -> Tz {
    *DISPLAY_TIMEZONE.get_or_init(|| tz)
}

/// The configured display timezone, or UTC if none was installed.
pub fn display_timezone() -> Tz {
    *DISPLAY_TIMEZONE.get().unwrap_or(&Tz::UTC)
}

/// SQL that pins a connection's session timezone.
///
/// `SET TimeZone` will not accept a bind parameter, so this uses `set_config`,
/// which will — the zone name never reaches the SQL text. `false` makes it
/// session-scoped rather than transaction-local.
pub const fn session_timezone_sql() -> &'static str {
    "SELECT set_config('TimeZone', $1, false)"
}

/// Render an instant as RFC 3339 in `tz`, carrying that zone's real offset.
///
/// Previously `timestamptz` was serialized as `DateTime<Utc>::to_rfc3339()`, so
/// every value claimed `+00:00` regardless of what the reader was shown. The
/// offset here is looked up per-instant, so DST transitions land correctly.
/// `AutoSi` keeps sub-second digits only when the value actually has them, so
/// microsecond-precision columns are not silently truncated on export.
pub fn format_timestamptz(value: DateTime<Utc>, tz: Tz) -> String {
    value
        .with_timezone(&tz)
        .to_rfc3339_opts(SecondsFormat::AutoSi, false)
}

/// Short label for the zone at a given instant (e.g. `+08`, `UTC`), for
/// annotating column headers and the CSV so a bare timestamp is not ambiguous.
pub fn offset_label(at: DateTime<Utc>, tz: Tz) -> String {
    if tz == Tz::UTC {
        return "UTC".to_string();
    }
    // %:z yields "+08:00"; trim a whole-hour offset to "+08" to stay narrow in
    // a column header, but keep the minutes for zones like +05:30.
    let offset = at.with_timezone(&tz).format("%:z").to_string();
    match offset.strip_suffix(":00") {
        Some(hours) => hours.to_string(),
        None => offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn instant(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0)
            .single()
            .expect("test instant should be unambiguous")
    }

    #[test]
    fn parses_iana_names() {
        assert_eq!(parse("Asia/Singapore").expect("should parse"), Tz::Asia__Singapore);
        assert_eq!(parse("UTC").expect("should parse"), Tz::UTC);
    }

    #[test]
    fn rejects_unknown_names() {
        let err = parse("Mars/Olympus").expect_err("unknown zone must be rejected");
        assert!(
            err.to_string().contains("Unknown timezone"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_offset_shorthand() {
        // "+08:00" is not an IANA name — accepting it would silently drop DST
        // rules for zones that have them.
        parse("+08:00").expect_err("offset shorthand must be rejected");
    }

    #[test]
    fn formats_with_real_offset_not_utc() {
        // 06:30Z is 14:30 in Singapore — the serialized value must say so
        // rather than claiming +00:00 while the grid renders 14:30.
        let formatted = format_timestamptz(instant(2026, 7, 30, 6, 30), Tz::Asia__Singapore);
        assert_eq!(formatted, "2026-07-30T14:30:00+08:00");
    }

    #[test]
    fn formats_utc_unchanged() {
        let formatted = format_timestamptz(instant(2026, 7, 30, 6, 30), Tz::UTC);
        assert_eq!(formatted, "2026-07-30T06:30:00+00:00");
    }

    #[test]
    fn offset_is_looked_up_per_instant_across_dst() {
        // London is +01:00 in July and +00:00 in January. A fixed offset
        // captured once at startup would get one of these wrong.
        let summer = format_timestamptz(instant(2026, 7, 30, 12, 0), Tz::Europe__London);
        let winter = format_timestamptz(instant(2026, 1, 30, 12, 0), Tz::Europe__London);
        assert_eq!(summer, "2026-07-30T13:00:00+01:00");
        assert_eq!(winter, "2026-01-30T12:00:00+00:00");
    }

    #[test]
    fn offset_label_trims_whole_hours() {
        assert_eq!(offset_label(instant(2026, 7, 30, 6, 30), Tz::Asia__Singapore), "+08");
        assert_eq!(offset_label(instant(2026, 7, 30, 6, 30), Tz::UTC), "UTC");
    }

    #[test]
    fn offset_label_keeps_half_hour_minutes() {
        assert_eq!(
            offset_label(instant(2026, 7, 30, 6, 30), Tz::Asia__Kolkata),
            "+05:30"
        );
    }

    #[test]
    fn display_timezone_defaults_to_utc_when_unset() {
        // Other tests in this binary may have installed a zone already, so this
        // only asserts the accessor never panics and yields a usable zone.
        let tz = display_timezone();
        assert!(!tz.name().is_empty());
    }
}
