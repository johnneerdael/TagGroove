# Learn

Learn one useful task at a time with copied practice files. The tutorials
explain what to do, what result to look for and why it works. You can follow
the first lessons in order or choose the one that matches your next task.

If you are new to tagging, start with [Your first tagging session](first-tagging-session.md).
It introduces selecting files, editing fields and checking a save. Later
lessons build on those habits rather than assuming you already know scripting.

## Choose a lesson

| If you want to… | Start here | You will learn… |
| --- | --- | --- |
| Correct an album's basic information | [Your first tagging session](first-tagging-session.md) | What a field is, how selection affects edits, and how to save deliberately. |
| Understand `%artist%` and `$num(...)` | [Format strings from zero](format-strings-from-zero.md) | How placeholders, functions, punctuation and optional parts produce text. |
| Make filenames match the tags | [Rename files and folders](rename-files-and-folders.md) | How to build and inspect a naming pattern before changing paths. |
| Recover information from filenames | [Get tags from filenames](tags-from-filenames.md) | How to describe a filename's parts and send each part to a field. |
| Plan a cleanup without changing everything at once | [Clean up a messy library](clean-a-messy-library.md) | How to narrow the selection and work through a repeatable cleanup. |
| Recognize and remove text patterns | [Regular expressions from zero](regex-from-zero.md) | How matching and replacement differ, with small examples. |
| Repeat several cleanup steps | [Build your first action group](first-action-group.md) | How ordered actions form a reusable group. |
| Review information from an online database | [Tag an album from an online database](tag-from-online-sources.md) | How to choose an edition, match files and control imported fields. |
| Put an image inside your audio files | [Give every album a cover](give-every-album-a-cover.md) | How embedded artwork differs from a folder image. |
| Turn tags into a report | [Export a CD catalogue](export-a-cd-catalogue.md) | How a template, loop, grouping key and output file work together. |

The invented Juniper Vale examples in the guide help you compare results
without needing someone else's music. The online-source lesson instead uses
an album you own and can identify: an invented teaching album cannot promise
a real database result.

## A route into scripting, regex and actions

These three tools solve different parts of a problem:

1. A **format string** builds text from known values. Learn it first so an
   expression such as `$num(%track%,2) - %title%` has a clear meaning.
2. A **regular expression** recognizes a pattern in text, such as a track
   number at the beginning of a title. Learn matching before replacement.
3. An **action group** runs chosen operations in order. Add one verified
   operation at a time so you can tell which step caused a result.

You do not need regular expressions to edit ordinary titles. You do not need
an action group for a one-off rename. Use the smallest tool that describes
the task clearly; the [Automate overview](../automate/index.md) helps choose it.

## Understand the ideas behind the controls

Concept pages answer questions that can make a correct operation look wrong:

| Question | Explanation |
| --- | --- |
| Why do the filename, tag title and player display disagree? | [What tags are](../concepts/what-are-tags.md) |
| Why can one file have several kinds of tag? | [Tag formats explained](../concepts/tag-formats.md) |
| Is `A; B` one artist value or two? | [Multi-value fields](../concepts/multi-value-fields.md) |
| What is the difference between `3/8` and a total field? | [Track and disc numbers](../concepts/track-and-disc-numbers.md) |
| Why does a folder image not appear in another player? | [Cover art](../concepts/cover-art.md) |
| Why are loudness tags different from changing the audio? | [ReplayGain and loudness](../concepts/replaygain.md) |
| Why does text contain garbled characters? | [Text encodings and codepages](../concepts/text-encodings.md) |
| How do album artists and sort names help a library? | [Sort fields and compilations](../concepts/sort-fields.md) |

## Practice with a result you can check

Keep originals separate. Start with one copied file or one album, follow the
lesson and compare its expected result before increasing the selection.
Check whether a command stages pending edits or writes immediately. Read
its result and available Undo operation rather than treating the preview
as a backup.

If the result differs, stop at that step. Compare the input field values,
selection, file order and exact expression with the lesson. Record any error
before trying again; repeating a whole cleanup can change already-correct
files a second time.

## Next steps

- [Your first tagging session](first-tagging-session.md)
- [Guides for everyday work](../guides/index.md)
- [Cookbook recipes for specific problems](../cookbook/index.md)
- [Reference for exact syntax and options](../reference/index.md)

<!-- Candidate evidence: docs/user-guide/_meta/planned-nav.yml and the linked existing lessons/concepts establish this learning route. Expressions are illustrative pointers to existing lessons, not newly executed examples. -->
