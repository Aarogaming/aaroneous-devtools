import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import markdown_fences as mf  # noqa: E402


class Blocks(unittest.TestCase):
    def test_top_level_blocks_come_back_in_order_with_their_labels(self):
        body = "a\n```json\n{}\n```\nb\n~~~\nplain\n~~~\n"
        self.assertEqual(mf.blocks(body), [("json", "{}", False), ("", "plain", False)])

    def test_a_longer_fence_swallows_an_inner_fence_and_only_a_matching_closer_ends_it(self):
        body = "````md\n```attestation\n{}\n```\n````\n```after\nx\n```"
        self.assertEqual(mf.blocks(body), [("md", "```attestation\n{}\n```", False), ("after", "x", True)])
        self.assertEqual(mf.blocks("~~~\n```x\n~~~\n"), [("", "```x", False)])
        self.assertEqual(mf.blocks("```\n~~~\n```"), [("", "~~~", False)])

    def test_a_closer_is_only_the_fence_character_and_whitespace(self):
        body = "```a\n```not a closer\nx\n```  \n"
        self.assertEqual(mf.blocks(body), [("a", "```not a closer\nx", False)])

    def test_an_unclosed_fence_is_not_a_block(self):
        self.assertEqual(mf.blocks("```attestation\n{}\n"), [])
        self.assertEqual(mf.blocks("```a\nx\n```\n```b\ny"), [("a", "x", False)])

    def test_inline_code_with_three_backticks_is_not_a_fence(self):
        self.assertEqual(mf.blocks("use ``` here\n``` `x`\nmore"), [])

    def test_html_comments_hide_blocks_until_their_closing_marker(self):
        self.assertEqual(mf.blocks("<!--\n```a\nx\n```\n-->\n```b\ny\n```"), [("b", "y", False)])
        self.assertEqual(mf.blocks("<!-- one line -->\n```b\ny\n```"), [("b", "y", False)])
        self.assertEqual(mf.blocks("<!--\n```a\nx\n```\n"), [], "an unterminated comment hides the rest")
        self.assertEqual(mf.blocks("text <!-- inline\n```a\nx\n```"), [("a", "x", False)], "only a line-start comment hides")

    def test_adjacency_means_only_blank_lines_between_two_blocks(self):
        self.assertEqual([b[2] for b in mf.blocks("```a\n1\n```\n\n\n```b\n2\n```")], [False, True])
        self.assertEqual([b[2] for b in mf.blocks("```a\n1\n```\ntext\n```b\n2\n```")], [False, False])

    def test_crlf_and_cr_line_endings_are_read_like_lf(self):
        self.assertEqual(mf.blocks("```a\r\nx\r\n```\r\n"), [("a", "x", False)])
        self.assertEqual(mf.blocks("```a\rx\r```"), [("a", "x", False)])


if __name__ == "__main__":
    unittest.main()
