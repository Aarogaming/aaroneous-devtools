"""The fenced code blocks of a markdown comment that a reader actually sees (lane C100).

A comment is read the way GitHub renders it: a fence opens a block, the block ends at the first line of at least as many
of the same character followed only by whitespace, and a ``` line inside a longer or different fence is plain text. An
HTML comment that starts a line hides everything up to its closing marker. A fence indented four or more spaces past its
container is indented code, not a fence; the container is the document root or the content of a list item. Reading
fences with a bare regex counted a verdict inside an example block (a ````markdown fence around a ```attestation fence),
inside ``<!-- ... -->``, or inside an indented code block as if it had been posted, and an HTML comment is invisible to
every human reader of the thread.
"""

import re

OPEN = re.compile(r"^(`{3,}|~{3,})(.*)$")
HTML_COMMENT_START = re.compile(r"^ {0,3}<!--")
LIST_ITEM = re.compile(r"^( *)([-*+]|\d{1,9}[.)])( +|$)")


def _indent(expanded):
    return len(expanded) - len(expanded.lstrip(" "))


def blocks(body):
    """Top-level fenced blocks in order, each ``(label, content, adjacent)``.

    ``label`` is the info string without trailing whitespace (a leading space still makes it a different label, as it
    always did here), ``content`` the text between the fences, and ``adjacent`` is True when only blank lines separate
    the block from the one before it (how an attestation and its signature are paired). A fence that is never closed is
    not a block, so a half-posted comment counts nothing; a backtick fence whose info string holds a backtick is inline
    code, not a fence. A list item's content indent makes a fence inside it count; a fence indented four or more spaces
    past its container is indented code and does not."""
    lines = body.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    found, i, clean, containers = [], 0, False, []
    while i < len(lines):
        line = lines[i]
        if HTML_COMMENT_START.match(line):
            end = i
            if "-->" not in line.split("<!--", 1)[1]:
                end = i + 1
                while end < len(lines) and "-->" not in lines[end]:
                    end += 1
            i, clean = end + 1, False
            continue
        expanded = line.expandtabs(4)
        if not expanded.strip():
            i += 1
            continue
        indent = _indent(expanded)
        while containers and containers[-1] > indent:
            containers.pop()
        base = containers[-1] if containers else 0
        text, container = expanded[indent:], base
        item = LIST_ITEM.match(expanded) if indent - base <= 3 else None
        if item:
            marker = item.group(2)
            width = len(item.group(3))
            width = width if 1 <= width <= 4 and len(expanded) > item.end() else 1
            content_indent = len(item.group(1)) + len(marker) + width
            containers.append(content_indent)
            text, container = expanded[content_indent:].lstrip(" "), content_indent
            indent = content_indent + (len(expanded[content_indent:]) - len(expanded[content_indent:].lstrip(" ")))
        opener = OPEN.match(text) if indent - container <= 3 else None
        if opener and not (opener.group(1)[0] == "`" and "`" in opener.group(2)):
            fence, info = opener.group(1), opener.group(2)
            closer = re.compile(r"^" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}[ ]*$")

            def closes(candidate):
                expanded_candidate = candidate.expandtabs(4)
                return (_indent(expanded_candidate) - container <= 3
                        and bool(closer.match(expanded_candidate.lstrip(" "))))

            end = next((j for j in range(i + 1, len(lines)) if closes(lines[j])), None)
            if end is None:
                break
            found.append((info.rstrip(), "\n".join(lines[i + 1:end]), clean))
            i, clean = end + 1, True
            continue
        clean = False
        i += 1
    return found
