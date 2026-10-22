# Conditions and logic

Use conditions to choose between results, test values and combine questions.
The format-string language tracks a **truth flag** alongside the visible
text. That flag controls `$if`, `$if2`, `$and`, `$or` and `$not`.

!!! tip "New to this?"
    Follow [Format strings from zero](../format-strings-from-zero.md)
    for a worked field fallback before using this reference.

!!! note "Bringing formulas from Mp3tag"
    Predicates produce empty text and update evaluation flags. Nested
    expressions and unused branches can affect those flags, so check imported
    formulas in Preview before applying them to tags or filenames. These rules
    do not establish full Mp3tag compatibility.

## Understand what counts as true

| Input to a condition | Flag |
|---|---|
| A placeholder with a nonempty first value | True |
| A missing or empty placeholder | False |
| A comparison that succeeds | True; displayed text is empty |
| A comparison that fails | False; displayed text is empty |
| Literal text, including `1`, `0` and `yes` | False |

Predicates such as `$eql`, `$and` and `$isdigit` produce **empty text**.
Use `$if($eql(03,3),1,0)` to display `1` or `0` explicitly. The literal digits
are selected output, not boolean inputs.

Evaluation proceeds from left to right within each expression. A nonempty
field lookup adds availability and truth; a predicate replaces both flags
with its answer, including flags established by earlier text in that same
expression. Thus `[%artist%$eql(1,2)]` disappears even when `ARTIST` is present.

Every function argument is evaluated in its own expression before a result
is chosen. Its flags contribute to the containing expression, including
arguments whose text is not selected. Most text and numeric functions do
not recompute truth from their output: a search in a present title can
return position text `0` while retaining a true flag. Test that position
with `$grtr(position,0)`.

An unused branch can also report an error. Do not hide an invalid expression
or division by zero in the unused branch of `$if`. Optional sections evaluate
their own children; when their final availability is false, they suppress
both their text and their flags.

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

Only the chosen branch supplies output text. Flags from the condition and
both branches still contribute. For example,
`$if($if(%year%,yes,no),present,missing)` gives `present` when `YEAR` exists,
even though the inner result `yes` is literal text. Likewise,
`$if($if(1,%artist%,no),present,missing)` gives `present` when `ARTIST`
exists: the unused inner branch contributes its flag. **Related:** [`$if2`](#if2).

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

**Sets the flag** to true when the values are equal under the rule above,
otherwise false.

| Argument | Meaning |
|---|---|
| `x`, `y` | The two values to compare |

| Format string | With these values | Result |
|---|---|---|
| `$if($eql(03,3),1,0)` | Any file | `1` |
| `$if($eql(3/8,3),1,0)` | Any file | `0` |
| `$if($eql($lower(%title%),paper boats),same,different)` | `TITLE` = `Paper Boats` | `same` |

`$eql(A,a)` is true because fallback string equality ignores case.
**Related:** [`$neql`](#neql), [`$strcmp`](search-compare.md#strcmp).

### `$neql(x,y)` { #neql }

**Sets the flag** to the inverse of `$eql`: true when unequal, otherwise
false.

| Argument | Meaning |
|---|---|
| `x`, `y` | The two values to compare |

| Format string | With these values | Result |
|---|---|---|
| `$if($neql(3,4),1,0)` | Any file | `1` |
| `$if($neql(A,a),1,0)` | Any file | `0` |
| `$if($neql($num(%track%,1),3),other,third)` | `TRACK` = `3/8` | `third` |

This does not mean "different after trimming"; surrounding whitespace
can change string comparison. **Related:** [`$eql`](#eql).

## Relational comparisons

The following four functions compare leading integers. A missing or
nonnumeric argument counts as 0. They are numeric comparisons, not
alphabetical sorting rules.

### `$grtr(x,y)` { #grtr }

**Sets the flag** to true when `x` is greater than `y`, otherwise false.

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$if($grtr(4,3),1,0)` | Any file | `1` |
| `$if($grtr(B2,0),1,0)` | Any file | `0` |
| `$if($grtr($strstr(%title%,Boats),0),found,missing)` | `TITLE` = `Paper Boats` | `found` |

Equal values are false. **Related:** [`$geql`](#geql).

### `$geql(x,y)` { #geql }

**Sets the flag** to true when `x` is greater than or equal to `y`, otherwise
false.

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$if($geql(3,3),1,0)` | Any file | `1` |
| `$if($geql(-1,0),1,0)` | Any file | `0` |
| `$if($geql($len(%title%),11),long enough,short)` | `TITLE` = `Paper Boats` | `long enough` |

The first example includes equality, which distinguishes it from `$grtr`.
**Related:** [`$leql`](#leql).

### `$less(x,y)` { #less }

**Sets the flag** to true when `x` is less than `y`, otherwise false.

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$if($less(2,3),1,0)` | Any file | `1` |
| `$if($less(3,3),1,0)` | Any file | `0` |
| `$if($less($num(%track%,1),4),first three,later)` | `TRACK` = `3/8` | `first three` |

Equal values are false. **Related:** [`$leql`](#leql).

### `$leql(x,y)` { #leql }

**Sets the flag** to true when `x` is less than or equal to `y`, otherwise
false.

| Argument | Meaning |
|---|---|
| `x`, `y` | Leading-integer inputs to compare |

| Format string | With these values | Result |
|---|---|---|
| `$if($leql(3,3),1,0)` | Any file | `1` |
| `$if($leql(4,3),1,0)` | Any file | `0` |
| `$if($leql($len(%title%),11),fits,too long)` | `TITLE` = `Paper Boats` | `fits` |

The equality case is included. **Related:** [`$less`](#less).

## Combine and invert flags

### `$and(x,y[,z…])` { #and }

**Sets the flag** to true only when every argument's flag is true. Accepts
2–63 arguments.

| Argument | Meaning |
|---|---|
| `x`, `y`, later arguments | Presence checks or boolean expressions |

| Format string | With these values | Result |
|---|---|---|
| `$if($and($eql(1,1),$grtr(3,1)),1,0)` | Any file | `1` |
| `$if($and(1,1),1,0)` | Any file | `0` |
| `$if($and(%artist%,%year%),complete,incomplete)` | `ARTIST` and `YEAR` present | `complete` |

Literal `1` is not a boolean true value. All arguments are evaluated;
this is not a short-circuit error guard. **Related:** [`$or`](#or).

### `$or(x,y[,z…])` { #or }

**Sets the flag** to true when at least one argument's flag is true. Accepts
2–63 arguments.

| Argument | Meaning |
|---|---|
| `x`, `y`, later arguments | Presence checks or boolean expressions |

| Format string | With these values | Result |
|---|---|---|
| `$if($or(%missing%,%artist%),1,0)` | `MISSING` absent; `ARTIST` present | `1` |
| `$if($or(yes,1),1,0)` | Any file | `0` |
| `$if($or(%artist%,%albumartist%),credited,unknown)` | `ARTIST` present; `ALBUMARTIST` absent | `credited` |

Use `$if2` when the goal is to return one field's actual text rather than
ask whether either field is available. **Related:** [`$and`](#and).

### `$not(x)` { #not }

**Sets the flag** to the opposite of `x`'s flag: false becomes true, and true
becomes false.

| Argument | Meaning |
|---|---|
| `x` | Flag to invert |

| Format string | With these values | Result |
|---|---|---|
| `$if($not(%missing%),1,0)` | `MISSING` absent | `1` |
| `$if($not(1),1,0)` | Any file | `1` |
| `$if($not($eql(%year%,2024)),other,year 2024)` | `YEAR` = `2024` | `year 2024` |

The second example follows the literal-text rule: literal `1` starts
false, so negating it is true. **Related:** [`$eql`](#eql).

### `$odd(x)` { #odd }

**Sets the flag** to true when the leading integer is odd, otherwise false.

| Argument | Meaning |
|---|---|
| `x` | Leading-integer input to test |

| Format string | With these values | Result |
|---|---|---|
| `$if($odd(3),1,0)` | Any file | `1` |
| `$if($odd(B2),1,0)` | Any file | `0` |
| `$if($odd($num(%track%,1)),odd track,even track)` | `TRACK` = `3/8` | `odd track` |

Negative odd numbers are also true. A nonnumeric value becomes 0 and is
therefore even under this rule. **Related:** [numbers and maths](numbers.md).

<!-- verified: crates/tag-core/src/script.rs:run, function, validate_arity; booleans are typed flags rather than displayed numeric text -->

## Related

- [Search and compare](search-compare.md)
- [Numbers and maths](numbers.md)
- [Format strings](../format-strings.md)
- [Scripting functions](index.md)
