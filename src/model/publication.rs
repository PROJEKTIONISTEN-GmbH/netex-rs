//! The publication: the root element and the frames it delivers.

use super::{
    Codespaces, DateTime, Duration, FrameDefaults, Id, KeyList, MultilingualString, Ref,
    ResourceFrame, ServiceCalendarFrame, ServiceFrame, SiteFrame, TimetableFrame, ValidBetween,
    ValidityConditions, Version,
};

netex_element! {
    /// The root of a NeTEx document.
    ///
    /// The `version` attribute is the schema version the document declares:
    /// `1.0` (the default), `1.1`, `ntx:1.1` (the German delivery), `2.0`.
    /// The 2.0 examples mostly still declare `1.0`, so the crate does not take
    /// the attribute's word for it; see [`crate::version::detect`].
    pub struct PublicationDelivery {
        attributes {
            /// The `version` attribute.
            pub version: Option<Box<str>> = "version",
        }
        children {
            /// When the document was produced.
            pub publication_timestamp: Option<DateTime> = "PublicationTimestamp",
            /// Who produced it.
            pub participant_ref: Option<Box<str>> = "ParticipantRef",
            /// How often it is refreshed.
            pub publication_refresh_interval: Option<Duration> = "PublicationRefreshInterval",
            /// What it holds.
            pub description: Option<MultilingualString> = "Description",
            /// The frames delivered.
            pub data_objects: Option<Frames> = "dataObjects",
        }
    }
}

netex_container! {
    /// A list of frames: the root's `dataObjects`, or a composite frame's `frames`.
    pub struct Frames of Frame {
        /// A frame holding other frames.
        Composite(CompositeFrame) = "CompositeFrame",
        /// Organisations, data sources, vehicle types.
        Resource(ResourceFrame) = "ResourceFrame",
        /// Stop places and other sites.
        Site(SiteFrame) = "SiteFrame",
        /// Lines, routes, stop points, journey patterns.
        Service(ServiceFrame) = "ServiceFrame",
        /// Day types and operating periods.
        ServiceCalendar(ServiceCalendarFrame) = "ServiceCalendarFrame",
        /// Journeys and passing times.
        Timetable(TimetableFrame) = "TimetableFrame",
    }
}

netex_element! {
    /// A frame holding other frames, with defaults they share.
    pub struct CompositeFrame {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The frame's validity conditions, or the conditions it defines.
            pub validity_conditions: Option<ValidityConditions> = "validityConditions",
            /// When the frame is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The frame's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name of the frame.
            pub name: Option<MultilingualString> = "Name",
            /// What the frame holds.
            pub description: Option<MultilingualString> = "Description",
            /// The profile the frame follows: `epip:EU_PI_LINE_OFFER`.
            pub type_of_frame_ref: Option<Ref> = "TypeOfFrameRef",
            /// The version frame this one derives from.
            pub baseline_version_frame_ref: Option<Ref> = "BaselineVersionFrameRef",
            /// Codespaces of the identifiers.
            pub codespaces: Option<Codespaces> = "codespaces",
            /// What the entities assume unless they say otherwise.
            pub frame_defaults: Option<FrameDefaults> = "FrameDefaults",
            /// The frames held.
            pub frames: Option<Frames> = "frames",
        }
    }
}

impl PublicationDelivery {
    /// The frames delivered, composite frames opened: every frame at any depth,
    /// in document order, the composite frames themselves included.
    pub fn frames(&self) -> impl Iterator<Item = &Frame> {
        let mut pending: Vec<&Frame> = self
            .data_objects
            .iter()
            .flat_map(|frames| frames.items.iter().rev())
            .collect();
        std::iter::from_fn(move || {
            let frame = pending.pop()?;
            if let Frame::Composite(composite) = frame {
                if let Some(frames) = &composite.frames {
                    pending.extend(frames.items.iter().rev());
                }
            }
            Some(frame)
        })
    }
}
