"""The fenced code blocks of a markdown comment that a reader actually sees (lane C100).

A comment is read the way GitHub renders it: a fence opens a block, the block ends at the first line of at least as many
of the same character followed only by whitespace, and a ``` line inside a longer or different fence is plain text. An
HTML comment that starts a line hides everything up to its closing marker. Reading fences with a bare regex counted a
verdict inside an example block (``````markdown`` around a ```attestation fence) or inside ``<!-- ... -->`` as if it
had been posted, and an HTML comment is invisible to every human reader of the thread.
"""

import re

OPEN = re.compile(r"^[ \t]*(`{3,}|~{3,})([^\n]*)$")
HTML_COMMENT_START = re.compile(r"^ {0,3}<!--")


def blocks(body):
    """Top-level fenced blocks in order, each ``(label, content, adjacent)``.

    ``label`` is the info string without trailing whitespace (a leading space still makes it a different label, as it
    always did here), ``content`` the text between the fences, and ``adjacent`` is True when only
    blank lines separate the block from the one before it (how an attestation and its signature are paired). A fence
    that is never closed is not a block, so a half-posted comment counts nothing; a backtick fence whose info string
    holds a backtick is inline code, not a fence."""
    lines = body.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    found, i, clean = [], 0, False
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
        opener = OPEN.match(line)
        if opener and not (opener.group(1)[0] == "`" and "`" in opener.group(2)):
            fence, info = opener.group(1), opener.group(2)
            closer = re.compile(r"^[ \t]*" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}[ \t]*$")
            end = next((j for j in range(i + 1, len(lines)) if closer.match(lines[j])), None)
            if end is None:
                break
            found.append((info.rstrip(),"\n".join(lines[i + 1:end]), clean))
            i, clean = end + 1, True
            continue
        if line.strip():
            clean = False
        i += 1
    return found
