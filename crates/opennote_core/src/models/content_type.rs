use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ContentType {
    Title,             // Only the first payload of a block has title
    HeadingLevel1,     // # Heading
    HeadingLevel2,     // ## Heading
    HeadingLevel3,     // ### Heading
    HeadingLevel4,     // #### Heading
    HeadingLevel5,     // ##### Heading
    HeadingLevel6,     // ###### Heading
    Bold,              // **bold text**
    Italic,            // *italic text*
    BoldAndItalic,     // ***bold and italic text***
    BlockQuotes,       // > block quote
    UnorderedListItem, // - item
    OrderedListItem,   // 1. item
    Code,              // `inline code`
    CodeBlock,         // ```code block```
    Horizontal,        // ---
    Link,              // [link text](url)
    URLAndEmail,       // <https://example.com>
    Reference,         // [link text][reference]
    Source,            // [reference]: https://example.com
    Image,             // ![alt text](image_url)
    JSONValue,         // A non-typed json value
    Body,
}
