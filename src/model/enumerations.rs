//! Enumerations, transcribed token by token from the schemas.
//!
//! Only the enumerations the typed elements carry are transcribed; every other
//! enumerated value is read as text. Each list is checked against both schema
//! releases by the `enumerations` test; where the releases differ, the
//! difference is in `version::table` and reported when a document is written
//! for the release that lacks the token.

netex_enum! {
    /// A mode of transport: the schema's `AllModesEnumeration`.
    pub enum VehicleMode as "AllModesEnumeration" {
        All = "all",
        AnyMode = "anyMode",
        Unknown = "unknown",
        Air = "air",
        Bus = "bus",
        TrolleyBus = "trolleyBus",
        Tram = "tram",
        Coach = "coach",
        Rail = "rail",
        IntercityRail = "intercityRail",
        UrbanRail = "urbanRail",
        Metro = "metro",
        Water = "water",
        Ferry = "ferry",
        Cableway = "cableway",
        Funicular = "funicular",
        Lift = "lift",
        SnowAndIce = "snowAndIce",
        Taxi = "taxi",
        SelfDrive = "selfDrive",
        Foot = "foot",
        Bicycle = "bicycle",
        Motorcycle = "motorcycle",
        Scooter = "scooter",
        Car = "car",
        Shuttle = "shuttle",
    }
}

netex_enum! {
    /// The kind of a stop place or scheduled stop point: the schema's `StopTypeEnumeration`.
    pub enum StopType as "StopTypeEnumeration" {
        OnstreetBus = "onstreetBus",
        OnstreetTram = "onstreetTram",
        Airport = "airport",
        RailStation = "railStation",
        MetroStation = "metroStation",
        BusStation = "busStation",
        CoachStation = "coachStation",
        TramStation = "tramStation",
        HarbourPort = "harbourPort",
        FerryPort = "ferryPort",
        FerryStop = "ferryStop",
        LiftStation = "liftStation",
        VehicleRailInterchange = "vehicleRailInterchange",
        TaxiRank = "taxiRank",
        Other = "other",
    }
}

netex_enum! {
    /// The kind of a quay: the schema's `QuayTypeEnumeration`.
    pub enum QuayType as "QuayTypeEnumeration" {
        AirlineGate = "airlineGate",
        RailPlatform = "railPlatform",
        RailIslandPlatform = "railIslandPlatform",
        RailPlatformSector = "railPlatformSector",
        MetroPlatform = "metroPlatform",
        MetroIslandPlatform = "metroIslandPlatform",
        MetroIslandPlatformSector = "metroIslandPlatformSector",
        CoachStop = "coachStop",
        BusStop = "busStop",
        BusPlatform = "busPlatform",
        BusBay = "busBay",
        TramPlatform = "tramPlatform",
        TramStop = "tramStop",
        BoatQuay = "boatQuay",
        FerryLanding = "ferryLanding",
        TelecabinPlatform = "telecabinPlatform",
        TaxiStand = "taxiStand",
        SetDownPlace = "setDownPlace",
        VehicleLoadingPlace = "vehicleLoadingPlace",
        HandicappedLevelBoardingSector = "handicappedLevelBoardingSector",
        Multimodal = "multimodal",
        BusStopWithinRoadwayBoarding = "busStopWithinRoadwayBoarding",
        TramStopWithinRoadwayBoarding = "tramStopWithinRoadwayBoarding",
        Other = "other",
    }
}

netex_enum! {
    /// The class of travel a capacity is for: the schema's `FareClassEnumeration`.
    pub enum FareClass as "FareClassEnumeration" {
        Unknown = "unknown",
        FirstClass = "firstClass",
        SecondClass = "secondClass",
        ThirdClass = "thirdClass",
        Preferente = "preferente",
        PremiumClass = "premiumClass",
        BusinessClass = "businessClass",
        StandardClass = "standardClass",
        Turista = "turista",
        EconomyClass = "economyClass",
        Any = "any",
    }
}

netex_enum! {
    /// The direction of a route or journey along its line: the schema's `DirectionTypeEnumeration`.
    pub enum DirectionType as "DirectionTypeEnumeration" {
        Inbound = "inbound",
        Outbound = "outbound",
        Clockwise = "clockwise",
        Anticlockwise = "anticlockwise",
    }
}

netex_enum! {
    /// Whether an accessibility feature is available: the schema's `LimitationStatusEnumeration`.
    pub enum LimitationStatus as "LimitationStatusEnumeration" {
        True = "true",
        False = "false",
        Unknown = "unknown",
        Partial = "partial",
    }
}

netex_enum! {
    /// The kind of an entrance: the schema's `EntranceEnumeration`.
    pub enum EntranceType as "EntranceEnumeration" {
        Opening = "opening",
        OpenDoor = "openDoor",
        Door = "door",
        SwingDoor = "swingDoor",
        RevolvingDoor = "revolvingDoor",
        AutomaticDoor = "automaticDoor",
        TicketBarrier = "ticketBarrier",
        Gate = "gate",
        Other = "other",
    }
}

netex_enum! {
    /// The kind of an access space: the schema's `AccessSpaceTypeEnumeration`.
    pub enum AccessSpaceType as "AccessSpaceTypeEnumeration" {
        Concourse = "concourse",
        BookingHall = "bookingHall",
        Forecourt = "forecourt",
        Underpass = "underpass",
        Overpass = "overpass",
        Passage = "passage",
        PassageSection = "passageSection",
        Lift = "lift",
        Gallery = "gallery",
        Garage = "garage",
        Shop = "shop",
        WaitingRoom = "waitingRoom",
        Restaurant = "restaurant",
        Other = "other",
        Staircase = "staircase",
        Wc = "wc",
    }
}

netex_enum! {
    /// How a transfer is made: the schema's `AccessModeEnumeration`.
    ///
    /// `wheelchair` and `animalRide` are 2.0 tokens; a 1.x document written
    /// with either loses it, and says so.
    pub enum AccessMode as "AccessModeEnumeration" {
        Foot = "foot",
        Bicycle = "bicycle",
        Boat = "boat",
        Car = "car",
        Taxi = "taxi",
        Shuttle = "shuttle",
        Ski = "ski",
        Skate = "skate",
        Motorcycle = "motorcycle",
        Scooter = "scooter",
        Wheelchair = "wheelchair",
        AnimalRide = "animalRide",
    }
}

netex_enum! {
    /// What a stop in a journey pattern is used for: the schema's `StopUseEnumeration`.
    pub enum StopUse as "StopUseEnumeration" {
        Access = "access",
        InterchangeOnly = "interchangeOnly",
        Passthrough = "passthrough",
        NoBoardingOrAlighting = "noBoardingOrAlighting",
    }
}

netex_enum! {
    /// Whether a stop point is a timing point: the schema's `TimingPointStatusEnumeration`.
    pub enum TimingPointStatus as "TimingPointStatusEnumeration" {
        TimingPoint = "timingPoint",
        SecondaryTimingPoint = "secondaryTimingPoint",
        NotTimingPoint = "notTimingPoint",
    }
}
