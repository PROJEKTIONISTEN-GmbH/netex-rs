//! Where a stop is, as the index keeps it: latitude and longitude in WGS 84.
//!
//! Without the `crs` feature, a location is where its `Latitude` and
//! `Longitude` say, and a link's geometry is judged as latitude and longitude
//! — what the index has always read. With it, a location may be a GML position
//! in the system its `srsName` names, else its frame's `DefaultLocationSystem`
//! ([`crate::crs`]), and a link's geometry is read in its system; a system
//! the crate does not know is a finding
//! ([`Concern::UnknownReferenceSystem`](super::Concern::UnknownReferenceSystem)).

use crate::model::Location;
#[cfg(feature = "crs")]
use crate::model::{Element, FrameDefaults};
use crate::stream::Scope;
#[cfg(feature = "crs")]
use crate::xml::{Child, Cursor};

/// The reference systems the frames of a document name as their default, as
/// the lean walk meets their `FrameDefaults`: each with the depth and the
/// identifier of the frame naming it.
#[derive(Debug, Default)]
pub(crate) struct FrameSystems {
    #[cfg(feature = "crs")]
    named: Vec<(usize, Option<crate::model::Id>, Box<str>)>,
}

impl FrameSystems {
    /// Notes the system the frame defaults `element` name, read from a copy
    /// of `cursor`, which stays where it is: the caller's sink reads them too.
    #[cfg(feature = "crs")]
    pub(crate) fn note<'i>(
        &mut self,
        cursor: &Cursor<'i>,
        element: &Child<'i>,
        scope: &Scope,
    ) -> crate::Result<()> {
        let mut copy = cursor.clone();
        let defaults = FrameDefaults::read(&mut copy, element)?;
        if let Some(system) = defaults.default_location_system {
            let depth = scope.frames().len();
            let frame = scope.frames().last().and_then(|frame| frame.id.clone());
            self.named.retain(|(named_at, _, _)| *named_at < depth);
            self.named.push((depth, frame, system));
        }
        Ok(())
    }

    /// The system named by the innermost frame of `scope` naming one.
    pub(crate) fn of(&self, scope: &Scope) -> Option<&str> {
        #[cfg(feature = "crs")]
        {
            self.named
                .iter()
                .rev()
                .find(|(depth, id, _)| {
                    scope
                        .frames()
                        .get(depth - 1)
                        .is_some_and(|frame| &frame.id == id)
                })
                .map(|(_, _, system)| &**system)
        }
        #[cfg(not(feature = "crs"))]
        {
            let _ = scope;
            None
        }
    }
}

/// Latitude and longitude in WGS 84 of a location, in a frame whose default
/// system is `default_system`; an error naming a system the crate does not know.
pub(crate) fn position(
    location: Option<&Location>,
    default_system: Option<&str>,
) -> Result<(Option<f64>, Option<f64>), String> {
    let Some(location) = location else {
        return Ok((None, None));
    };
    #[cfg(feature = "crs")]
    {
        match crate::crs::location_in_wgs84(location, default_system) {
            Ok(Some((latitude, longitude))) => Ok((Some(latitude), Some(longitude))),
            Ok(None) => Ok((None, None)),
            Err(unknown) => Err(unknown.0),
        }
    }
    #[cfg(not(feature = "crs"))]
    {
        let _ = default_system;
        Ok((location.latitude, location.longitude))
    }
}

/// The two readings of a link geometry's first position, `first` as written,
/// as latitude and longitude: the one its system states, and the other; an
/// error naming a system the crate does not know. `geometry` is the
/// geometry's source text, for the system it names.
pub(crate) fn readings(
    first: (f64, f64),
    geometry: &str,
    default_system: Option<&str>,
) -> Result<[(f64, f64); 2], String> {
    #[cfg(feature = "crs")]
    {
        let named = crate::geometry::geometry(geometry).and_then(|g| g.srs_name);
        let system =
            crate::crs::system_of(named.as_deref(), default_system).map_err(|unknown| unknown.0)?;
        Ok([
            system.to_wgs84(first.0, first.1),
            system.to_wgs84(first.1, first.0),
        ])
    }
    #[cfg(not(feature = "crs"))]
    {
        let _ = (geometry, default_system);
        Ok([first, (first.1, first.0)])
    }
}
