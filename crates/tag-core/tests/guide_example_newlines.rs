//! Keep the complete guide example corpus portable across LF and CRLF checkouts.
mod guide_examples;
use guide_examples::text_blocks;

#[test]
fn written_page_fences_have_the_same_literal_contents_in_both_checkout_formats() {
    for (page, expected_count) in [
        (
            include_str!("../../../docs/user-guide/scripting/format-strings.md"),
            10,
        ),
        (
            include_str!("../../../docs/user-guide/scripting/regular-expressions.md"),
            6,
        ),
        (
            include_str!("../../../docs/user-guide/getting-started/first-tagging-session.md"),
            1,
        ),
        (
            include_str!(
                "../../../docs/user-guide/converters/recipes/rename-zero-padded-track-numbers.md"
            ),
            5,
        ),
        (
            include_str!(
                "../../../docs/user-guide/converters/recipes/tags-from-folder-and-filename.md"
            ),
            8,
        ),
        (
            include_str!("../../../docs/user-guide/actions/recipes/feat-artists-to-title.md"),
            3,
        ),
        (
            include_str!("../../../docs/user-guide/converters/tag-to-filename.md"),
            2,
        ),
        (
            include_str!("../../../docs/user-guide/converters/filename-to-tag.md"),
            3,
        ),
        (
            include_str!("../../../docs/user-guide/converters/filename-to-filename.md"),
            4,
        ),
        (
            include_str!("../../../docs/user-guide/converters/tag-to-tag.md"),
            1,
        ),
        (
            include_str!("../../../docs/user-guide/converters/text-file-to-tag.md"),
            7,
        ),
        (
            include_str!("../../../docs/user-guide/converters/auto-numbering.md"),
            0,
        ),
        (
            include_str!("../../../docs/user-guide/scripting/format-strings-from-zero.md"),
            10,
        ),
        (
            include_str!("../../../docs/user-guide/scripting/regex-from-zero.md"),
            7,
        ),
        (
            include_str!("../../../docs/user-guide/converters/rename-files-and-folders.md"),
            5,
        ),
        (
            include_str!("../../../docs/user-guide/actions/export.md"),
            2,
        ),
    ] {
        let lf = page.replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        let expected = text_blocks(&lf);
        let title = page.lines().find(|line| line.starts_with("# ")).unwrap();
        assert_eq!(expected.len(), expected_count, "fence count for {title}");
        assert_eq!(text_blocks(&crlf), expected, "LF/CRLF contents for {title}");
    }
}

#[test]
fn fence_indentation_is_removed_without_trimming_literal_example_spaces() {
    for newline in ["\n", "\r\n"] {
        let page = ["    ```text", "      literal  ", "    ", "    ```"].join(newline);
        assert_eq!(text_blocks(&page), ["  literal  \n"]);
    }
}
