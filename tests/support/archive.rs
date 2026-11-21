//! Zip archives written byte by byte, for what a well-behaved writer will not
//! write: a member whose central directory claims another size than it holds,
//! two members of one name, a name that is no path.

/// One member of an archive, stored, under its name as given.
pub struct Member<'a> {
    pub name: &'a [u8],
    pub text: &'a [u8],
    /// The inflated size the central directory claims, in a zip64 field; the
    /// member's own size where none is given.
    pub claimed: Option<u64>,
}

impl<'a> Member<'a> {
    pub fn new(name: &'a str, text: &'a [u8]) -> Self {
        Member {
            name: name.as_bytes(),
            text,
            claimed: None,
        }
    }

    /// A member whose name is not UTF-8 — or not text at all.
    pub fn named_by_bytes(name: &'a [u8], text: &'a [u8]) -> Self {
        Member {
            name,
            text,
            claimed: None,
        }
    }

    /// The same member, its central directory claiming `size` bytes inflated.
    pub fn claiming(mut self, size: u64) -> Self {
        self.claimed = Some(size);
        self
    }
}

/// The archive of `members`, in their order.
pub fn stored(members: &[Member<'_>]) -> Vec<u8> {
    let mut local = Vec::new();
    let mut central = Vec::new();
    for member in members {
        let crc = crc32fast::hash(member.text);
        let size = u32::try_from(member.text.len()).expect("a small member");
        let name_length = u16::try_from(member.name.len()).expect("a short name");
        let offset = u32::try_from(local.len()).expect("a small archive");
        // Version needed: 4.5 where a zip64 field states the size, else 2.0.
        let version: u16 = if member.claimed.is_some() { 45 } else { 20 };

        local.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        local.extend_from_slice(&version.to_le_bytes());
        local.extend_from_slice(&[0; 8]); // flags, stored, time, date
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&name_length.to_le_bytes());
        local.extend_from_slice(&0_u16.to_le_bytes());
        local.extend_from_slice(member.name);
        local.extend_from_slice(member.text);

        let zip64: Vec<u8> = member.claimed.map_or_else(Vec::new, |claimed| {
            let mut field = Vec::new();
            field.extend_from_slice(&0x0001_u16.to_le_bytes());
            field.extend_from_slice(&8_u16.to_le_bytes());
            field.extend_from_slice(&claimed.to_le_bytes());
            field
        });
        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&version.to_le_bytes()); // made by
        central.extend_from_slice(&version.to_le_bytes()); // needed
        central.extend_from_slice(&[0; 8]); // flags, stored, time, date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes()); // compressed
        let uncompressed = if member.claimed.is_some() {
            u32::MAX // in the zip64 field
        } else {
            size
        };
        central.extend_from_slice(&uncompressed.to_le_bytes());
        central.extend_from_slice(&name_length.to_le_bytes());
        central.extend_from_slice(
            &u16::try_from(zip64.len())
                .expect("a short field")
                .to_le_bytes(),
        );
        central.extend_from_slice(&[0; 6]); // comment, disk, internal attributes
        central.extend_from_slice(&0_u32.to_le_bytes()); // external attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(member.name);
        central.extend_from_slice(&zip64);
    }

    let count = u16::try_from(members.len()).expect("few members");
    let mut end = Vec::new();
    end.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    end.extend_from_slice(&[0; 4]); // disks
    end.extend_from_slice(&count.to_le_bytes());
    end.extend_from_slice(&count.to_le_bytes());
    end.extend_from_slice(&u32::try_from(central.len()).expect("small").to_le_bytes());
    end.extend_from_slice(&u32::try_from(local.len()).expect("small").to_le_bytes());
    end.extend_from_slice(&0_u16.to_le_bytes());

    [local, central, end].concat()
}
