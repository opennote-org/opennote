# Parser test document

A plain paragraph. It has two sentences! Does punctuation split it?

Another paragraph with **bold**, *italic*, ~~strikethrough~~, and `inline code`.
This line follows a soft break.
This line ends with two spaces.  
This line follows a hard break.

中文段落。这里有逗号，问号？还有感叹号！Emoji: 🦀🌍

## Links and images

A [link](https://example.com/page?q=hello.world) inside a paragraph.

![Standalone image](./assets/photo.png "Image title")

Text before ![inline image](https://example.com/image.png?width=640&height=480) and text after.

![Reference image][photo]

A [reference link][website].

## Nested structures

- First item with **bold text**
- Second item
  - Nested item with ![small image](./assets/small.png)
  - Another nested item

1. First numbered item
2. Second numbered item

> A block quote.
>
> Another quoted paragraph with *emphasis*.
>
> - A quoted list item

## Code

```rust
// This should remain code, not become an image payload.
let markdown = "![fake image](./not-an-image.png)";
println!("Hello. World?");
```

    Indented code with **literal formatting**.
    ![another fake image](./also-not-an-image.png)

## Standalone events

---

Paragraph after the horizontal rule.

<div>
HTML block with <img src="./assets/html-image.png" alt="HTML image">
</div>

## Optional Markdown extensions

- [ ] Unchecked task
- [x] Checked task

| Name | Content |
| --- | --- |
| Text | **Bold cell** |
| Image | ![cell image](./assets/cell.png) |

A footnote reference.[^note]

[^note]: Footnote text with a [link](https://example.com).

## Escapes and entities

\![escaped image syntax](./not-an-image.png)

Escaped \*asterisks\*, an entity &amp;, and an autolink <https://example.com>.

## Final paragraph

Image references below must survive reconstruction even though their definitions do not render as paragraphs.

[photo]: https://example.com/reference.png "Reference image title"
[website]: https://example.com "Website title"
