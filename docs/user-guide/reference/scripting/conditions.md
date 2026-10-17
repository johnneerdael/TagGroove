# Conditions and logic

Use conditions to choose between results, test values and combine questions.
The format-string language tracks a **truth flag** alongside the visible
text. That flag controls `$if`, `$if2`, `$and`, `$or` and `$not`.

!!! tip "New to this?"
    Follow [Format strings from zero](../../learn/format-strings-from-zero.md)
    for a worked field fallback before using this reference.

## Understand what counts as true

| Input to a condition | Flag |
|---|---|
| A placeholder with a nonempty first value | True |
| A missing or empty placeholder | False |
| A comparison that succeeds | True; displayed text is `1` |
| A comparison that fails | False; displayed text is `0` |
| Literal text, including `1`, `0` and `yes` | False |

Most text and numeric functions inherit flags from their arguments. They
do not recompute truth from their output. For example, searching a present
title can return the position text `0` while still carrying a true flag.
Test that position with `$grtr(position,0)`.

All function arguments are evaluated before a result is chosen. A branch
that is not selected can still report an error. Do not hide an invalid
expression or division by zero in the unused branch of `$if`.

Examples use `ARTIST` = `Juniper Vale`, `TITLE` = `Paper Boats`, `TRACK` =
`3/8`, `YEAR` = `2024`, with `ALBUMARTIST` and `MISSING` absent.

### `$if(condition,yes,no)` { #if }

**Returns** `yes` when the condition flag is true, otherwise `no`.

| Argument | Meaning |
|---|---|
| `condition` | A field-presence test or boolean expression |
| `yes`, `no` | Results for the two cases; either can be empty |

| Format string | With these values | Result |
|---|---|---|
| `$if(%year%,dated,undated)` | `YEAR` = `2024` | `dated` |
| `$if(1,yes,no)` | Any file | `no` |
| `$if($eql(%year%,2024),current,other)` | `YEAR` = `2024` | `current` |

The result inherits the chosen branch's flag. Choosing literal `yes`
displays that word but does not make it true if passed into another
condition. **Related:** [`$if2`](#if2).

### `$if2(x,fallback)` { #if2 }

**Returns** `x` when its flag is true, otherwise `fallback`.

| Argument | Meaning |
|---|---|
| `x` | Preferred expression, often a field placeholder |
| `fallback` | Expression to use when `x` is false |

| Format string | With these values | Result |
|---|---|---|
| `$if2(%albumartist%,%artist%)` | `ALBUMARTIST` absent; `ARTIST` = `Juniper Vale` | `Juniper Vale` |
| `$if2(ready,fallback)` | Any file | `fallback` |
| `$if2($upper(%title%),unknown)` | `TITLE` = `Paper Boats` | `PAPER BOATS` |

This is a reliable missing-field fallback when the first argument is a
placeholder. It is not a general nonempty-string test: literal `ready`
is false, and a transformed present field can retain a true flag even
when its output becomes empty. **Related:** [`$iflonger`](#iflonger).

### `$ifgreater(x,y,yes,no)` { #ifgreater }

**Returns** `yes` if the leading integer in `x` is greater than the one in
`y`, otherwise `no`.

| Argument | Meaning |
|---|---|
| `x`, `y` | Numeric inputs, interpreted with the leading-integer rule |
| `yes`, `no` | Results to select |

| Format string | With these values | Result |
|---|---|---|
| `$ifgreater(%track%,2,later,early)` | `TRACK` = `3/8` | `later` |
| `$ifgreater(B2,0,positive,zero or negative)` | Any file | `zero or negative` |
| `$ifgreater($len(%title%),10,long,short)` | `TITLE` = `Paper Boats` | `long` |

Equal inputs take the `no` branch. Nonnumeric input becomes 0 rather than
causing an error. **Related:** [`$grtr`](#grtr), [numeric rules](numbers.md).

### `$iflonger(x,n,yes,no)` { #iflonger }

**Returns** `yes` when `x` contains more than `n` characters, otherwise `no`.

| Argument | Meaning |
|---|---|
| `x` | Text whose character count is tested |
| `n` | Threshold; negative values become 0 |
| `yes`, `no` | Results to select |

| Format string | With these values | Result |
|---|---|---|
| `$iflonger(%title%,10,long,short)` | `TITLE` = `Paper Boats` | `long` |
| `$iflonger(abc,3,long,short)` | Any file | `short` |
| `$iflonger($trim('   '),0,present,empty)` | Any file | `empty` |

Length counts Unicode characters, not UTF-8 bytes. This gives an
explicit nonempty-text check when `n` is 0, independently of inherited
truth flags. **Related:** [`$if2`](#if2), [text functions](text.md#len).

## Equality

`$eql` and `$neql` compare integers when both complete input strings parse
as integers. Otherwise they compare lowercase text. Unlike relational
functions, they do not discard a suffix such as `/8` for integer equality.
Use `$strcmp` for exact string equality including case and zero padding.

### `$eql(x,y)` { #eql }

**Returns** true (`1`) when the values are equal under the rule above,
otherwise false (`0`).

| Argument | Meaning |
|---|---|
| `x`, `y` | The two values to compare |

| Format string | With these values | Result |
|---|---|---|
| `$eql(03,3)` | Any file | `1` |
| `$eql(3/8,3)` | Any file | `0` |
| `$if($eql($lower(%title%),paper boats),same,different)` | `TITLE` = `Paper Boats` | `same` |

`$eql(A,a)` is true because fallback string equality ignores case.
**Related:** [`$neql`](#neql), [`$strcmp`](search-compare.md#strcmp).

### `$neql(x,y)` { #neql }

**Returns** the inverse of `$eql`: true (`1`) when unequal, otherwise
false (`0`).

| Argument | Meaning |
|---|---|
| `x`, `y` | The two values to compare |

| Format string | With these values | Result |
|---|---|---|
| `$neql(3,4)` | Any file | `1` |
| `$neql(A,a)` | Any file | `0` |
| `$if($neql($num(%track%,1),3),other,third)` | `TRACK` = `3/8` | `third` |

This does not mean "different after trimming"; surrounding whitespace
can change string comparison. **Related:** [`$eql`](#eql).

## Relational comparisons

The following four functions compare leading integers. A missing or
nonnumeric argument counts as 0. They are numeric comparisons, not
alphabetical sorting rules.

### `$grtr(x,y)` { #grtr }

**Returns** true (`1`) when `x` is greater than `y`, otherwise false (`0`).

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$grtr(4,3)` | Any file | `1` |
| `$grtr(B2,0)` | Any file | `0` |
| `$if($grtr($strstr(%title%,Boats),0),found,missing)` | `TITLE` = `Paper Boats` | `found` |

Equal values are false. **Related:** [`$geql`](#geql).

### `$geql(x,y)` { #geql }

**Returns** true (`1`) when `x` is greater than or equal to `y`, otherwise
false (`0`).

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$geql(3,3)` | Any file | `1` |
| `$geql(-1,0)` | Any file | `0` |
| `$if($geql($len(%title%),11),long enough,short)` | `TITLE` = `Paper Boats` | `long enough` |

The first example includes equality, which distinguishes it from `$grtr`.
**Related:** [`$leql`](#leql).

### `$less(x,y)` { #less }

**Returns** true (`1`) when `x` is less than `y`, otherwise false (`0`).

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$less(2,3)` | Any file | `1` |
| `$less(3,3)` | Any file | `0` |
| `$if($less($num(%track%,1),4),first three,later)` | `TRACK` = `3/8` | `first three` |

Equal values are false. **Related:** [`$leql`](#leql).

### `$leql(x,y)` { #leql }

**Returns** true (`1`) when `x` is less than or equal to `y`, otherwise
false (`0`).

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$leql(3,3)` | Any file | `1` |
| `$leql(4,3)` | Any file | `0` |
| `$if($leql($len(%title%),11),fits,too long)` | `TITLE` = `Paper Boats` | `fits` |

The equality case is included. **Related:** [`$less`](#less).

## Combine and invert flags

### `$and(x,y[,z…])` { #and }

**Returns** true (`1`) only when every argument's flag is true. Accepts
2–63 arguments.

| Argument | Meaning |
|---|---|
| `x`, `y`, later arguments | Presence checks or boolean expressions |

| Format string | With these values | Result |
|---|---|---|
| `$and($eql(1,1),$grtr(3,1))` | Any file | `1` |
| `$and(1,1)` | Any file | `0` |
| `$if($and(%artist%,%year%),complete,incomplete)` | `ARTIST` and `YEAR` present | `complete` |

Literal `1` is not a boolean true value. All arguments are evaluated;
this is not a short-circuit error guard. **Related:** [`$or`](#or).

### `$or(x,y[,z…])` { #or }

**Returns** true (`1`) when at least one argument's flag is true. Accepts
2–63 arguments.

| Argument | Meaning |
|---|---|
| `x`, `y`, later arguments | Presence checks or boolean expressions |

| Format string | With these values | Result |
|---|---|---|
| `$or(%missing%,%artist%)` | `MISSING` absent; `ARTIST` present | `1` |
| `$or(yes,1)` | Any file | `0` |
| `$if($or(%artist%,%albumartist%),credited,unknown)` | `ARTIST` present; `ALBUMARTIST` absent | `credited` |

Use `$if2` when the goal is to return one field's actual text rather than
ask whether either field is available. **Related:** [`$and`](#and).

### `$not(x)` { #not }

**Returns** the opposite of `x`'s flag: false becomes true (`1`), and true
becomes false (`0`).

| Argument | Meaning |
|---|---|
| `x` | Flag to invert |

| Format string | With these values | Result |
|---|---|---|
| `$not(%missing%)` | `MISSING` absent | `1` |
| `$not(1)` | Any file | `1` |
| `$if($not($eql(%year%,2024)),other,year 2024)` | `YEAR` = `2024` | `year 2024` |

The second example follows the literal-text rule: literal `1` starts
false, so negating it is true. **Related:** [`$eql`](#eql).

### `$odd(x)` { #odd }

**Returns** true (`1`) when the leading integer is odd, otherwise false
(`0`).

| Argument | Meaning |
|---|---|
| `x` | Leading-integer input to test |

| Format string | With these values | Result |
|---|---|---|
| `$odd(3)` | Any file | `1` |
| `$odd(B2)` | Any file | `0` |
| `$if($odd($num(%track%,1)),odd track,even track)` | `TRACK` = `3/8` | `odd track` |

Negative odd numbers are also true. A nonnumeric value becomes 0 and is
therefore even under this rule. **Related:** [numbers and maths](numbers.md).

<!-- verified: crates/tag-core/src/script.rs:run, function, validate_arity; booleans are typed flags rather than displayed numeric text -->

## Related

- [Search and compare](search-compare.md)
- [Numbers and maths](numbers.md)
- [Format strings](../format-strings.md)
- [Scripting functions](index.md)
