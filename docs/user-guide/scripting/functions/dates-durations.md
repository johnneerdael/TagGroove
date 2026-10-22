# Dates and durations

Use `$fmtdate` for calendar timestamps and `$fmtduration` for elapsed time.
A duration is not a date: 90061 seconds can display as `25:01:01` without
referring to a particular day.

!!! tip "New to this?"
    Read [Format strings from zero](../format-strings-from-zero.md)
    for argument syntax and nested expressions.

Function names are case-insensitive: `$fmtDate` and `$fmtdate` call the
same function. Tokens inside a date pattern are case-sensitive.

### `$fmtdate(pattern[,timestamp])` { #fmtdate }

**Returns** a calendar date/time formatted with `pattern`. Supply an
integer Unix timestamp in seconds, or omit it to use the current time.

| Argument | Meaning |
|---|---|
| `pattern` | Date tokens and literal text, described below |
| `timestamp` | Optional whole seconds since 1970-01-01 00:00:00 UTC; an explicitly empty argument means 0 |

The timestamp selects an instant. The application's timezone determines
its displayed local calendar date and time. Month and weekday names use
the date locale. The examples below explicitly assume **UTC** and
**en-US**, with timestamp `1704164645` = 2024-01-02 03:04:05 UTC.

| Format string | With these values | Result |
|---|---|---|
| `$fmtdate('yyyy-MM-dd',1704164645)` | UTC, en-US | `2024-01-02` |
| `$fmtdate('yyyy-MM-dd',)` | UTC, en-US | `1970-01-01` |
| `$upper($fmtdate(MMM,1704164645))` | UTC, en-US | `JAN` |
| `$fmtdate('yyyy-MM-dd ''day'' HH:mm',1704164645)` | UTC, en-US | `2024-01-02 day 03:04` |

To format a file timestamp, use a raw timestamp placeholder, for example
`$fmtdate('yyyy-MM-dd',%_file_mod_datetime_raw%)`. A human-readable date
string is not an integer timestamp. Invalid integers, unsupported named
locales and timestamps outside the supported calendar range report errors.

#### Every date token

Results in this table use the same UTC/en-US instant:

| Token | Meaning | Example result |
|---|---|---|
| `yyyy` | Four-digit year | `2024` |
| `yy` | Two-digit year | `24` |
| `MMMM` | Full month name | `January` |
| `MMM` | Abbreviated month name | `Jan` |
| `MM` | Two-digit month | `01` |
| `M` | Month without leading zero | `1` |
| `dddd` | Full weekday name | `Tuesday` |
| `EEEE` | Alias for full weekday name | `Tuesday` |
| `ddd` | Abbreviated weekday name | `Tue` |
| `EE` | Alias for abbreviated weekday name | `Tue` |
| `dd` | Two-digit day of month | `02` |
| `d` | Day of month without leading zero | `2` |
| `HH` | Two-digit 24-hour clock hour | `03` |
| `H` | 24-hour clock hour without leading zero | `3` |
| `hh` | Two-digit 12-hour clock hour | `03` |
| `h` | 12-hour clock hour without leading zero | `3` |
| `mm` | Two-digit minute | `04` |
| `m` | Minute without leading zero | `4` |
| `ss` | Two-digit second | `05` |
| `s` | Second without leading zero | `5` |

Use `MM` for a month and `mm` for minutes. Neither `%Y-%m-%d` nor an
arbitrary platform date-format language is substituted for this token set.
Text not recognized as a token is copied, but a token embedded in an
ordinary word is still interpreted.

#### Quote literal date text

There are two parsing layers. First the format-string parser reads the
function argument. Then the date formatter reads tokens within that
argument.

In the last expression above, the outer quotes group the pattern as one
argument. Each doubled apostrophe inside those quotes passes one
apostrophe to the date formatter, which sees:

```text
yyyy-MM-dd 'day' HH:mm
```

The date formatter copies `day` literally. Without its own quotes, the
`d` in `day` would become the day-of-month token. Within a date pattern,
two apostrophes represent one literal apostrophe. An unclosed date literal
is an error.

### `$fmtduration(x[,unit[,hours[,milliseconds]]])` { #fmtduration }

**Returns** elapsed time in minutes/seconds, optionally including hours
and milliseconds. It accepts a finite decimal number.

| Argument | Meaning |
|---|---|
| `x` | Duration value; a missing or empty value means zero |
| `unit` | `s` for seconds, `m` for minutes, or `ms` for milliseconds; omitted/empty means seconds |
| `hours` | Literal `1` requests `HH:MM:SS`; other values leave total minutes as `M:SS` |
| `milliseconds` | Literal `1` appends `.mmm`; omitted/other values do not |

| Format string | With these values | Result |
|---|---|---|
| `$fmtduration(61)` | Any file | `1:01` |
| `$fmtduration(-1.25,s,0,1)` | Any file | `-0:01.250` |
| `$fmtduration($add(3600,61),s,1)` | Any file | `01:01:01` |
| `$fmtduration(1.5,m)` | Any file | `1:30` |
| `$fmtduration(61234,ms,0,1)` | Any file | `1:01.234` |
| `$fmtduration(90061,s,1)` | Any file | `25:01:01` |

Hours can exceed 23 because this is a duration. The function does not
insert day units. The `hours` and `milliseconds` options compare their
text to `1`; they are not the typed condition flags used by `$if`.

Fractional precision beyond milliseconds is truncated, not rounded.
Units must be lowercase `m`, `s` or `ms`. Scientific notation, `NaN`,
infinity, malformed decimal text and arithmetic overflow report errors.
The integer arithmetic functions use a different leading-integer rule;
do not pass a decimal through `$add` if its fractional part must survive.

<!-- verified: crates/tag-core/src/script.rs:format_date, local_datetime, format_duration, decimal_milliseconds -->

## Related

- [Numbers and maths](numbers.md)
- [Placeholders](../placeholders.md)
- [Format strings](../format-strings.md)
- [Scripting functions](index.md)
