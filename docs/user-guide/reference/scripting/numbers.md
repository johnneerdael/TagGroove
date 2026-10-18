# Numbers and maths

Use these functions to pad track numbers, calculate integers and format
readable counts. They return text for the enclosing format string.

!!! tip "New to this?"
    Start with [Format strings from zero](../../learn/format-strings-from-zero.md).
    The [format-string reference](../format-strings.md) explains nesting and quoting.

## Read the numeric rules first

Arithmetic uses signed 64-bit integers, from `-9223372036854775808` through
`9223372036854775807`, rather than decimal arithmetic. A leading integer
is read after any initial whitespace. Text after that integer is ignored:
`3/8` means 3, `12px` means 12, and `2.9` means 2. Text without a leading
integer, such as `B2`, means 0. Do not apply these functions to vinyl sides
or decimal measurements without first deciding how to represent them.

TagGroove treats an input prefix outside that range as 0, just like text
without a usable leading integer. This conversion happens **before** the
operation. By contrast, an operation whose result overflows the range reports
an error, as does division by zero. Operations run from left to right.

<!-- numeric-overflow-examples:start -->
| Expression | What happens | Outcome |
|---|---|---|
| `$add(9223372036854775808,1)` | The first input cannot fit, so it becomes 0 before adding 1 | `1` |
| `$add(9223372036854775807,1)` | Both inputs fit, but their sum does not | Error: `Integer overflow or division by zero` |
<!-- numeric-overflow-examples:end -->

These are TagGroove's current conversion and calculation rules. Check the
input before treating a plausible result as proof that every number was
read successfully. Numeric output does not automatically become a true
condition; use comparison functions when testing a result.

<!-- verified: crates/tag-core/tests/guide_scripting_details.rs:published_numeric_examples_distinguish_conversion_from_operation_overflow -->

### `$num(x,n)` { #num }

**Returns** the leading integer from `x`, padded with zeros to at least
`n` characters. Use it for a predictable filename width.

| Argument | Meaning |
|---|---|
| `x` | Numeric text or a field such as `%track%` |
| `n` | Minimum width; a negative value is treated as zero |

| Format string | With these values | Result |
|---|---|---|
| `$num(%track%,2)` | `TRACK` = `3/8` | `03` |
| `$num(B2,2)` | Any file | `00` |
| `$num($add(%track%,10),3)` | `TRACK` = `3/8` | `013` |

The total after `/` is not retained. Width is a minimum, so
`$num(123,2)` remains `123`. Excessive padding is rejected by the output
limit. **Related:** [Auto-numbering recipe](../../cookbook/rename-zero-padded-track-numbers.md).

### `$add(x,y[,z…])` { #add }

**Returns** the sum of 2–64 integer arguments.

| Argument | Meaning |
|---|---|
| `x`, `y`, later arguments | Integers or text with a leading integer |

| Format string | With these values | Result |
|---|---|---|
| `$add(3,4)` | Any file | `7` |
| `$add(B2,4)` | Any file | `4` |
| `$num($add(%track%,8),2)` | `TRACK` = `3/8` | `11` |

`$add(1.9,2.9)` calculates `1 + 2`, not a decimal sum. A missing field
contributes 0. **Related:** [`$sub`](#sub), [`$num`](#num).

### `$sub(x,y[,z…])` { #sub }

**Returns** the result of subtracting each later argument from the running
value. Accepts 2–64 arguments.

| Argument | Meaning |
|---|---|
| `x` | Initial value |
| `y`, later arguments | Values to subtract in sequence |

| Format string | With these values | Result |
|---|---|---|
| `$sub(10,3,2)` | Any file | `5` |
| `$sub(2,5)` | Any file | `-3` |
| `$num($sub(%track%,1),2)` | `TRACK` = `3/8` | `02` |

The first example means `(10 - 3) - 2`. Negative results are allowed;
they do not indicate a parsing error. **Related:** [`$add`](#add).

### `$mul(x,y[,z…])` { #mul }

**Returns** the product of 2–64 integer arguments.

| Argument | Meaning |
|---|---|
| `x`, `y`, later arguments | Factors multiplied in order |

| Format string | With these values | Result |
|---|---|---|
| `$mul(2,3,4)` | Any file | `24` |
| `$mul(5,%missing%)` | `MISSING` absent | `0` |
| `$add($mul(%track%,10),1)` | `TRACK` = `3/8` | `31` |

One missing or nonnumeric argument can turn the entire product into 0.
Large products can overflow and report an error. **Related:** [`$div`](#div).

### `$div(x,y[,z…])` { #div }

**Returns** sequential integer division, discarding each fractional part
toward zero. Accepts 2–64 arguments.

| Argument | Meaning |
|---|---|
| `x` | Initial dividend |
| `y`, later arguments | Divisors applied from left to right |

| Format string | With these values | Result |
|---|---|---|
| `$div(17,5)` | Any file | `3` |
| `$div(-17,5)` | Any file | `-3` |
| `$add($div(125,60),1)` | Any file | `3` |

Division by 0, including a missing field used as divisor, is an error.
Use `$fmtduration` for a formatted time rather than trying to build time
text with rounded decimal division. **Related:** [`$mod`](#mod),
[dates and durations](dates-durations.md).

### `$mod(x,y[,z…])` { #mod }

**Returns** the remainder after each integer division, from left to right.
Accepts 2–64 arguments.

| Argument | Meaning |
|---|---|
| `x` | Initial dividend |
| `y`, later arguments | Divisors for successive remainder operations |

| Format string | With these values | Result |
|---|---|---|
| `$mod(17,5)` | Any file | `2` |
| `$mod(-17,5)` | Any file | `-2` |
| `$num($mod(125,60),2)` | Any file | `05` |

A negative dividend can produce a negative remainder. This is a remainder
operation, not a guarantee of a positive mathematical modulo. A zero
divisor is an error. **Related:** [`$div`](#div).

### `$fmtnum(x)` { #fmtnum }

**Returns** a grouped integer using the application's numeric locale or
system number-format settings. Use it for display, not identifiers.

| Argument | Meaning |
|---|---|
| `x` | Text interpreted with the leading-integer rule |

These examples use an `en-US` numeric format:

| Format string | With these values | Result |
|---|---|---|
| `$fmtnum(1234567)` | Numeric locale `en-US` | `1,234,567` |
| `$fmtnum(B2)` | Numeric locale `en-US` | `0` |
| `$fmtnum($mul(1250,8))` | Numeric locale `en-US` | `10,000` |

Another locale can use different group separators or grouping widths.
An unsupported locale or invalid custom grouping reports an error. Use
[`$num`](#num) for zero padding without locale grouping.

### `$rand()` { #rand }

**Returns** a newly generated integer from 0 through 32767 inclusive.
There are no arguments, but the empty parentheses are required.

| Format string | With these values | Result |
|---|---|---|
| `$rand()` | Any file | An integer from 0 through 32767 |
| `$num($rand(),5)` | Any file | Five decimal digits, for example `00123` |
| `$mod($rand(),10)` | Any file | An integer from 0 through 9 |

The examples describe ranges, not fixed outputs. Each call can return a
different value, including two calls in one expression. Random numbers
are not unique IDs and can collide; do not use a single random value as
the only distinguishing part of a filename. **Related:** [`$uuid`](lists-metadata.md#uuid).

<!-- verified: crates/tag-core/src/script.rs:number, function, validate_arity; examples require en-US where stated -->

## Related

- [Conditions and logic](conditions.md)
- [Dates and durations](dates-durations.md)
- [Format strings](../format-strings.md)
- [Scripting functions](index.md)
