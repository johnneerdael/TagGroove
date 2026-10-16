"""Keep unfinished user-guide pages out of both HTML and search indexes."""

from mkdocs.utils.meta import get_data


def on_files(files, config):
    for file in list(files.documentation_pages()):
        _, metadata = get_data(file.content_string)
        if metadata.get("status") == "draft":
            files.remove(file)
    return files
