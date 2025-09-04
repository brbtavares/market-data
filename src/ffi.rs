//! # FFI da ProfitDLL
//!
//! Definições Rust para funções, tipos e constantes expostas pela ProfitDLL.dll,
//! permitindo chamadas seguras/idiomáticas (carregamento estático e dinâmico).
//!
//! ## Casos de uso
//! - Registrar callbacks e traduzir estruturas C/Delphi -> Rust
//! - Invocar APIs de ordem/posição/market data
//!
//! ## Notas
//! - Assinaturas e layout baseados no MANUAL.md da DLL
//! - Assegurar ABI "system" e strings UTF-16 (PWideChar)
//!
//! Notes
//! - All exported functions use stdcall on 32-bit and Microsoft x64 on 64-bit. In Rust, use the "system" ABI.
//! - Delphi PWideChar maps to UTF-16 pointer. We use *const u16 for input and *mut u16 for output buffers.
//! - Integer=i32, Cardinal=u32, Byte=u8, Int64=i64, Double=f64, BOOL=i32.
//! - TSystemTime matches WinAPI SYSTEMTIME layout.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

#[cfg(not(windows))]
compile_error!("This FFI is only supported on Windows.");

pub type BOOL = i32;
pub type LPARAM = isize;
pub type Byte = u8;
pub type Cardinal = u32;
pub type Int64_ = i64; // Avoid name clash with Rust literals
pub type TString0In = *mut u16; // buffer for 0-terminated wide strings (output)

// Wide string pointers
pub type PWideChar = *const u16;
pub type PWStrMut = *mut u16;

// Error/Result type used by some APIs (HRESULT-like, 32-bit signed)
pub type NResult = i32;

// Flags
pub type TFlags = u32;

// Delphi/Win SYSTEMTIME
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TSystemTime {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}

// Basic records
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TAssetIDRec {
    pub pwcTicker: PWideChar,
    pub pwcBolsa: PWideChar,
    pub nFeed: i32,
}
pub type PAssetIDRec = *const TAssetIDRec;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TAccountRec {
    pub pwhAccountID: PWideChar,
    pub pwhTitular: PWideChar,
    pub pwhNomeCorretora: PWideChar,
    pub nCorretoraID: i32,
}
pub type PAccountRec = *const TAccountRec;

// Enums (explicit discriminants)
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorOrderType {
    cotMarket = 1,
    cotLimit = 2,
    cotStopLimit = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorOrderSide {
    cosBuy = 1,
    cosSell = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorPositionType {
    cptDayTrade = 1,
    cptConsolidated = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorOrderStatus {
    cosNew = 0,
    cosPartiallyFilled = 1,
    cosFilled = 2,
    cosDoneForDay = 3,
    cosCanceled = 4,
    cosReplaced = 5,
    cosPendingCancel = 6,
    cosStopped = 7,
    cosRejected = 8,
    cosSuspended = 9,
    cosPendingNew = 10,
    cosCalculated = 11,
    cosExpired = 12,
    cosAcceptedForBidding = 13,
    cosPendingReplace = 14,
    cosPartiallyFilledCanceled = 15,
    cosReceived = 16,
    cosPartiallyFilledExpired = 17,
    cosPartiallyFilledRejected = 18,
    cosUnknown = 200,
    cosHadesCreated = 201,
    cosBrokerSent = 202,
    cosClientCreated = 203,
    cosOrderNotCreated = 204,
    cosCanceledByAdmin = 205,
    cosDelayFixGateway = 206,
    cosScheduledOrder = 207,
}

// Legacy enums (pre 4.0.0.18)
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorOrderTypeV0 {
    cotLimit = 0,
    cotStop = 1,
    cotMarket = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorOrderSideV0 {
    cosBuy = 0,
    cosSell = 1,
}

// Account identifier
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TConnectorAccountIdentifier {
    pub Version: u8, // V0
    pub BrokerID: i32,
    pub AccountID: PWideChar,
    pub SubAccountID: PWideChar,
    pub Reserved: i64,
}
pub type PConnectorAccountIdentifier = *const TConnectorAccountIdentifier;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorAccountIdentifierOut {
    pub Version: u8, // V0
    pub BrokerID: i32,
    pub AccountID: TString0In,
    pub AccountIDLength: i32,
    pub SubAccountID: TString0In,
    pub SubAccountIDLength: i32,
    pub Reserved: i64,
}
pub type PConnectorAccountIdentifierOut = *mut TConnectorAccountIdentifierOut;
pub type PConnectorAccountIdentifierArrayOut = *mut TConnectorAccountIdentifierOut; // array head

// Asset identifier
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TConnectorAssetIdentifier {
    pub Version: u8, // V0
    pub Ticker: PWideChar,
    pub Exchange: PWideChar,
    pub FeedType: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorAssetIdentifierOut {
    pub Version: u8, // V0
    pub Ticker: PWideChar,
    pub TickerLength: i32,
    pub Exchange: PWideChar,
    pub ExchangeLength: i32,
    pub FeedType: u8,
}

// Order identifier
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorOrderIdentifier {
    pub Version: u8, // V0
    pub LocalOrderID: i64,
    pub ClOrderID: PWideChar,
}

// Send/Change/Cancel records
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorSendOrder {
    pub Version: u8, // V0 or V1
    pub AccountID: TConnectorAccountIdentifier,
    pub AssetID: TConnectorAssetIdentifier,
    pub Password: PWideChar,
    pub OrderType: u8, // see version note
    pub OrderSide: u8, // see version note
    pub Price: f64,
    pub StopPrice: f64,
    pub Quantity: i64,
}
pub type PConnectorSendOrder = *const TConnectorSendOrder;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorChangeOrder {
    pub Version: u8, // V0
    pub AccountID: TConnectorAccountIdentifier,
    pub OrderID: TConnectorOrderIdentifier,
    pub Password: PWideChar,
    pub Price: f64,
    pub StopPrice: f64,
    pub Quantity: i64,
}
pub type PConnectorChangeOrder = *const TConnectorChangeOrder;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorCancelOrder {
    pub Version: u8, // V0
    pub AccountID: TConnectorAccountIdentifier,
    pub OrderID: TConnectorOrderIdentifier,
    pub Password: PWideChar,
}
pub type PConnectorCancelOrder = *const TConnectorCancelOrder;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorCancelOrders {
    pub Version: u8, // V0
    pub AccountID: TConnectorAccountIdentifier,
    pub AssetID: TConnectorAssetIdentifier,
    pub Password: PWideChar,
}
pub type PConnectorCancelOrders = *const TConnectorCancelOrders;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorCancelAllOrders {
    pub Version: u8, // V0
    pub AccountID: TConnectorAccountIdentifier,
    pub Password: PWideChar,
}
pub type PConnectorCancelAllOrders = *const TConnectorCancelAllOrders;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorZeroPosition {
    pub Version: u8, // V0..V1
    pub AccountID: TConnectorAccountIdentifier,
    pub AssetID: TConnectorAssetIdentifier,
    pub Password: PWideChar,
    pub Price: f64,
    // V1
    pub PositionType: u8, // TConnectorPositionType
}
pub type PConnectorZeroPosition = *const TConnectorZeroPosition;

// Account types
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TConnectorAccountType {
    cutOwner = 0,
    cutAssessor = 1,
    cutMaster = 2,
    cutSubAccount = 3,
    cutRiskMaster = 4,
    cutPropOffice = 5,
    cutPropManager = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorTradingAccountOut {
    pub Version: u8,
    // In
    pub AccountID: TConnectorAccountIdentifier,
    // Out
    pub BrokerName: PWideChar,
    pub BrokerNameLength: i32,
    pub OwnerName: PWideChar,
    pub OwnerNameLength: i32,
    pub SubOwnerName: PWideChar,
    pub SubOwnerNameLength: i32,
    pub AccountFlags: TFlags,
    // V1
    pub AccountType: u8, // TConnectorAccountType
}
pub type PConnectorTradingAccountOut = *mut TConnectorTradingAccountOut;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TConnectorTradingAccountPosition {
    pub Version: u8,
    // In
    pub AccountID: TConnectorAccountIdentifier,
    pub AssetID: TConnectorAssetIdentifier,
    // Out
    pub OpenQuantity: i64,
    pub OpenAveragePrice: f64,
    pub OpenSide: u8,
    pub DailyAverageSellPrice: f64,
    pub DailySellQuantity: i64,
    pub DailyAverageBuyPrice: f64,
    pub DailyBuyQuantity: i64,
    pub DailyQuantityD1: i64,
    pub DailyQuantityD2: i64,
    pub DailyQuantityD3: i64,
    pub DailyQuantityBlocked: i64,
    pub DailyQuantityPending: i64,
    pub DailyQuantityAlloc: i64,
    pub DailyQuantityProvision: i64,
    pub DailyQuantity: i64,
    pub DailyQuantityAvailable: i64,
    // V1
    pub PositionType: u8,
    // V2
    pub EventID: i64,
}
pub type PConnectorTradingAccountPosition = *mut TConnectorTradingAccountPosition;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorOrder {
    pub Version: u8,
    pub OrderID: TConnectorOrderIdentifier,
    pub AccountID: TConnectorAccountIdentifier,
    pub AssetID: TConnectorAssetIdentifier,
    pub Quantity: i64,
    pub TradedQuantity: i64,
    pub LeavesQuantity: i64,
    pub Price: f64,
    pub StopPrice: f64,
    pub AveragePrice: f64,
    pub OrderSide: u8,
    pub OrderType: u8,
    pub OrderStatus: u8,
    pub ValidityType: u8,
    pub Date: TSystemTime,
    pub LastUpdate: TSystemTime,
    pub CloseDate: TSystemTime,
    pub ValidityDate: TSystemTime,
    pub TextMessage: PWideChar,
    // V1
    pub EventID: i64,
}
pub type PConnectorOrder = *const TConnectorOrder;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorOrderOut {
    pub Version: u8,
    // In
    pub OrderID: TConnectorOrderIdentifier,
    // Out
    pub AccountID: TConnectorAccountIdentifierOut,
    pub AssetID: TConnectorAssetIdentifierOut,
    pub Quantity: i64,
    pub TradedQuantity: i64,
    pub LeavesQuantity: i64,
    pub Price: f64,
    pub StopPrice: f64,
    pub AveragePrice: f64,
    pub OrderSide: u8,
    pub OrderType: u8,
    pub OrderStatus: u8,
    pub ValidityType: u8,
    pub Date: TSystemTime,
    pub LastUpdate: TSystemTime,
    pub CloseDate: TSystemTime,
    pub ValidityDate: TSystemTime,
    pub TextMessage: PWideChar,
    pub TextMessageLength: i32,
    // V1
    pub EventID: i64,
}
pub type PConnectorOrderOut = *mut TConnectorOrderOut;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TConnectorTrade {
    pub Version: u8,
    pub TradeDate: TSystemTime,
    pub TradeNumber: u32, // Cardinal
    pub Price: f64,
    pub Quantity: i64,
    pub Volume: f64,
    pub BuyAgent: i32,
    pub SellAgent: i32,
    pub TradeType: u8, // TTradeType (not enumerated here)
}
pub type PConnectorTrade = *mut TConnectorTrade;

// Callback types (use Option to allow NULL function pointers)
pub type TStateCallback = Option<unsafe extern "system" fn(nConnStateType: i32, value: i32)>;
pub type TProgressCallback =
    Option<unsafe extern "system" fn(rAssetID: TAssetIDRec, nProgress: i32)>;
pub type TNewTradeCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        pwcDate: PWideChar,
        nTradeNumber: u32,
        dPrice: f64,
        dVol: f64,
        nQtd: i32,
        nBuyAgent: i32,
        nSellAgent: i32,
        nTradeType: i32,
        bEdit: u8,
    ),
>;
pub type TNewDailyCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        pwcDate: PWideChar,
        dOpen: f64,
        dHigh: f64,
        dLow: f64,
        dClose: f64,
        dVol: f64,
        dAjuste: f64,
        dMaxLimit: f64,
        dMinLimit: f64,
        dVolBuyer: f64,
        dVolSeller: f64,
        nQtd: i32,
        nNegocios: i32,
        nContratosOpen: i32,
        nQtdBuyer: i32,
        nQtdSeller: i32,
        nNegBuyer: i32,
        nNegSeller: i32,
    ),
>;
pub type TPriceBookCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nAction: i32,
        nPosition: i32,
        nSide: i32,
        nQtds: i32,
        nCount: i32,
        dPrice: f64,
        pArraySell: *const core::ffi::c_void,
        pArrayBuy: *const core::ffi::c_void,
    ),
>;
pub type TPriceBookCallbackV2 = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nAction: i32,
        nPosition: i32,
        nSide: i32,
        nQtds: i64,
        nCount: i32,
        dPrice: f64,
        pArraySell: *const core::ffi::c_void,
        pArrayBuy: *const core::ffi::c_void,
    ),
>;
pub type TOfferBookCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nAction: i32,
        nPosition: i32,
        nSide: i32,
        nQtd: i32,
        nAgent: i32,
        nOfferID: i64,
        dPrice: f64,
        bHasPrice: u8,
        bHasQtd: u8,
        bHasDate: u8,
        bHasOfferID: u8,
        bHasAgent: u8,
        pwcDate: PWideChar,
        pArraySell: *const core::ffi::c_void,
        pArrayBuy: *const core::ffi::c_void,
    ),
>;
pub type TOfferBookCallbackV2 = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nAction: i32,
        nPosition: i32,
        nSide: i32,
        nQtd: i64,
        nAgent: i32,
        nOfferID: i64,
        dPrice: f64,
        bHasPrice: u8,
        bHasQtd: u8,
        bHasDate: u8,
        bHasOfferID: u8,
        bHasAgent: u8,
        pwcDate: PWideChar,
        pArraySell: *const core::ffi::c_void,
        pArrayBuy: *const core::ffi::c_void,
    ),
>;
pub type TConnectorAssetPositionListCallback = Option<
    unsafe extern "system" fn(
        AccountID: TConnectorAccountIdentifier,
        AssetID: TConnectorAssetIdentifier,
        EventID: i64,
    ),
>;
pub type TAccountCallback = Option<
    unsafe extern "system" fn(
        nCorretora: i32,
        CorretoraNomeCompleto: PWideChar,
        AccountID: PWideChar,
        NomeTitular: PWideChar,
    ),
>;
pub type TOrderChangeCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nCorretora: i32,
        nQtd: i32,
        nTradedQtd: i32,
        nLeavesQtd: i32,
        nSide: i32,
        dPrice: f64,
        dStopPrice: f64,
        dAvgPrice: f64,
        nProfitID: i64,
        TipoOrdem: PWideChar,
        Conta: PWideChar,
        Titular: PWideChar,
        ClOrdID: PWideChar,
        Status: PWideChar,
        Date: PWideChar,
        TextMessage: PWideChar,
    ),
>;
pub type THistoryCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nCorretora: i32,
        nQtd: i32,
        nTradedQtd: i32,
        nLeavesQtd: i32,
        nSide: i32,
        dPrice: f64,
        dStopPrice: f64,
        dAvgPrice: f64,
        nProfitID: i64,
        TipoOrdem: PWideChar,
        Conta: PWideChar,
        Titular: PWideChar,
        ClOrdID: PWideChar,
        Status: PWideChar,
        Date: PWideChar,
    ),
>;
pub type THistoryTradeCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        pwcDate: PWideChar,
        nTradeNumber: u32,
        dPrice: f64,
        dVol: f64,
        nQtd: i32,
        nBuyAgent: i32,
        nSellAgent: i32,
        nTradeType: i32,
    ),
>;
pub type TTinyBookCallback =
    Option<unsafe extern "system" fn(rAssetID: TAssetIDRec, dPrice: f64, nQtd: i32, nSide: i32)>;
pub type TAssetListCallback =
    Option<unsafe extern "system" fn(rAssetID: TAssetIDRec, pwcName: PWideChar)>;
pub type TAssetListInfoCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        pwcName: PWideChar,
        pwcDescription: PWideChar,
        nMinOrderQtd: i32,
        nMaxOrderQtd: i32,
        nLote: i32,
        stSecurityType: i32,
        ssSecuritySubType: i32,
        dMinPriceIncrement: f64,
        dContractMultiplier: f64,
        strValidDate: PWideChar,
        strISIN: PWideChar,
    ),
>;
pub type TAssetListInfoCallbackV2 = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        pwcName: PWideChar,
        pwcDescription: PWideChar,
        nMinOrderQtd: i32,
        nMaxOrderQtd: i32,
        nLote: i32,
        stSecurityType: i32,
        ssSecuritySubType: i32,
        dMinPriceIncrement: f64,
        dContractMultiplier: f64,
        strValidDate: PWideChar,
        strISIN: PWideChar,
        strSetor: PWideChar,
        strSubSetor: PWideChar,
        strSegmento: PWideChar,
    ),
>;
pub type TChangeStateTicker =
    Option<unsafe extern "system" fn(rAssetID: TAssetIDRec, pwcDate: PWideChar, nState: i32)>;
pub type TInvalidTickerCallback =
    Option<unsafe extern "system" fn(AssetID: TConnectorAssetIdentifier)>;
pub type TAdjustHistoryCallback = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        dValue: f64,
        strAdjustType: PWideChar,
        strObserv: PWideChar,
        dtAjuste: PWideChar,
        dtDeliber: PWideChar,
        dtPagamento: PWideChar,
        nAffectPrice: i32,
    ),
>;
pub type TAdjustHistoryCallbackV2 = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        dValue: f64,
        strAdjustType: PWideChar,
        strObserv: PWideChar,
        dtAjuste: PWideChar,
        dtDeliber: PWideChar,
        dtPagamento: PWideChar,
        nFlags: u32,
        dMult: f64,
    ),
>;
pub type TTheoreticalPriceCallback = Option<
    unsafe extern "system" fn(rAssetID: TAssetIDRec, dTheoreticalPrice: f64, nTheoreticalQtd: i64),
>;
pub type TChangeCotation = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        pwcDate: PWideChar,
        nTradeNumber: u32,
        dPrice: f64,
    ),
>;
pub type THistoryCallbackV2 = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nCorretora: i32,
        nQtd: i32,
        nTradedQtd: i32,
        nLeavesQtd: i32,
        nSide: i32,
        nValidity: i32,
        dPrice: f64,
        dStopPrice: f64,
        dAvgPrice: f64,
        nProfitID: i64,
        TipoOrdem: PWideChar,
        Conta: PWideChar,
        Titular: PWideChar,
        ClOrdID: PWideChar,
        Status: PWideChar,
        LastUpdate: PWideChar,
        CloseDate: PWideChar,
        ValidityDate: PWideChar,
    ),
>;
pub type TOrderChangeCallbackV2 = Option<
    unsafe extern "system" fn(
        rAssetID: TAssetIDRec,
        nCorretora: i32,
        nQtd: i32,
        nTradedQtd: i32,
        nLeavesQtd: i32,
        nSide: i32,
        nValidity: i32,
        dPrice: f64,
        dStopPrice: f64,
        dAvgPrice: f64,
        nProfitID: i64,
        TipoOrdem: PWideChar,
        Conta: PWideChar,
        Titular: PWideChar,
        ClOrdID: PWideChar,
        Status: PWideChar,
        LastUpdate: PWideChar,
        CloseDate: PWideChar,
        ValidityDate: PWideChar,
        TextMessage: PWideChar,
    ),
>;
pub type TConnectorOrderCallback =
    Option<unsafe extern "system" fn(a_OrderID: TConnectorOrderIdentifier)>;
pub type TConnectorAccountCallback =
    Option<unsafe extern "system" fn(a_AccountID: TConnectorAccountIdentifier)>;
pub type TConnectorTradeCallback = Option<
    unsafe extern "system" fn(
        a_Asset: TConnectorAssetIdentifier,
        a_pTrade: *const core::ffi::c_void,
        a_nFlags: u32,
    ),
>;

// Enumerator callback
pub type TConnectorEnumerateOrdersProc =
    Option<unsafe extern "system" fn(a_Order: *const TConnectorOrder, a_Param: LPARAM) -> BOOL>;
// Asset enumerator callback (const TConnectorAssetIdentifier; LPARAM) -> BOOL
pub type TConnectorEnumerateAssetProc =
    Option<unsafe extern "system" fn(a_Asset: TConnectorAssetIdentifier, a_Param: LPARAM) -> BOOL>;

// Constants
pub const CA_IS_SUB_ACCOUNT: TFlags = 1;
pub const CA_IS_ENABLED: TFlags = 2;

pub const CM_IS_SHORT_NAME: TFlags = 1;

// Exchanges (char codes)
pub const gc_bvBCB: i32 = 65; // A
pub const gc_bvBovespa: i32 = 66; // B
pub const gc_bvCambio: i32 = 68; // D
pub const gc_bvEconomic: i32 = 69; // E
pub const gc_bvBMF: i32 = 70; // F
pub const gc_bvMetrics: i32 = 75; // K
pub const gc_bvCME: i32 = 77; // M
pub const gc_bvNasdaq: i32 = 78; // N
pub const gc_bvOXR: i32 = 79; // O
pub const gc_bvPioneer: i32 = 80; // P
pub const gc_bvDowJones: i32 = 88; // X
pub const gc_bvNyse: i32 = 89; // Y

// Connection/Login states
pub const CONNECTION_STATE_LOGIN: i32 = 0;
pub const CONNECTION_STATE_ROTEAMENTO: i32 = 1;
pub const CONNECTION_STATE_MARKET_DATA: i32 = 2;
pub const CONNECTION_STATE_MARKET_LOGIN: i32 = 3;

pub const LOGIN_CONNECTED: i32 = 0;
pub const LOGIN_INVALID: i32 = 1;
pub const LOGIN_INVALID_PASS: i32 = 2;
pub const LOGIN_BLOCKED_PASS: i32 = 3;
pub const LOGIN_EXPIRED_PASS: i32 = 4;
pub const LOGIN_UNKNOWN_ERR: i32 = 200;

pub const ROTEAMENTO_DISCONNECTED: i32 = 0;
pub const ROTEAMENTO_CONNECTING: i32 = 1;
pub const ROTEAMENTO_CONNECTED: i32 = 2;
pub const ROTEAMENTO_BROKER_DISCONNECTED: i32 = 3;
pub const ROTEAMENTO_BROKER_CONNECTING: i32 = 4;
pub const ROTEAMENTO_BROKER_CONNECTED: i32 = 5;

pub const MARKET_DISCONNECTED: i32 = 0;
pub const MARKET_CONNECTING: i32 = 1;
pub const MARKET_WAITING: i32 = 2;
pub const MARKET_NOT_LOGGED: i32 = 3;
pub const MARKET_CONNECTED: i32 = 4;

pub const CONNECTION_ACTIVATE_VALID: i32 = 0;
pub const CONNECTION_ACTIVATE_INVALID: i32 = 1;

// Trade callback flags
pub const TC_IS_EDIT: u32 = 1;
pub const TC_LAST_PACKET: u32 = 2;

// OfferBook footer flag
pub const OB_LAST_PACKET: u32 = 1;

// NL_* error codes
pub const NL_OK: i32 = 0x0000_0000u32 as i32;
pub const NL_INTERNAL_ERROR: i32 = 0x8000_0001u32 as i32;
pub const NL_NOT_INITIALIZED: i32 = 0x8000_0002u32 as i32;
pub const NL_INVALID_ARGS: i32 = 0x8000_0003u32 as i32;
pub const NL_WAITING_SERVER: i32 = 0x8000_0004u32 as i32;
pub const NL_NO_LOGIN: i32 = 0x8000_0005u32 as i32;
pub const NL_NO_LICENSE: i32 = 0x8000_0006u32 as i32;
pub const NL_OUT_OF_RANGE: i32 = 0x8000_0009u32 as i32;
pub const NL_MARKET_ONLY: i32 = 0x8000_000Au32 as i32;
pub const NL_NO_POSITION: i32 = 0x8000_000Bu32 as i32;
pub const NL_NOT_FOUND: i32 = 0x8000_000Cu32 as i32;
pub const NL_VERSION_NOT_SUPPORTED: i32 = 0x8000_000Du32 as i32;
pub const NL_OCO_NO_RULES: i32 = 0x8000_000Eu32 as i32;
pub const NL_EXCHANGE_UNKNOWN: i32 = 0x8000_000Fu32 as i32;
pub const NL_NO_OCO_DEFINED: i32 = 0x8000_0010u32 as i32;
pub const NL_INVALID_SERIE: i32 = 0x8000_0011u32 as i32;
pub const NL_LICENSE_NOT_ALLOWED: i32 = 0x8000_0012u32 as i32;
pub const NL_NOT_HARD_LOGOUT: i32 = 0x8000_0013u32 as i32;
pub const NL_SERIE_NO_HISTORY: i32 = 0x8000_0014u32 as i32;
pub const NL_ASSET_NO_DATA: i32 = 0x8000_0015u32 as i32;
pub const NL_SERIE_NO_DATA: i32 = 0x8000_0016u32 as i32;
pub const NL_HAS_STRATEGY_RUNNING: i32 = 0x8000_0017u32 as i32;
pub const NL_SERIE_NO_MORE_HISTORY: i32 = 0x8000_0018u32 as i32;
pub const NL_SERIE_MAX_COUNT: i32 = 0x8000_0019u32 as i32;
pub const NL_DUPLICATE_RESOURCE: i32 = 0x8000_001Au32 as i32;
pub const NL_UNSIGNED_CONTRACT: i32 = 0x8000_001Bu32 as i32;
pub const NL_NO_PASSWORD: i32 = 0x8000_001Cu32 as i32;
pub const NL_NO_USER: i32 = 0x8000_001Du32 as i32;
pub const NL_FILE_ALREADY_EXISTS: i32 = 0x8000_001Eu32 as i32;
pub const NL_INVALID_TICKER: i32 = 0x8000_001Fu32 as i32;
pub const NL_NOT_MASTER_ACCOUNT: i32 = 0x8000_0020u32 as i32;

// Link to the DLL and declare extern functions (only when not using dynamic loading)
#[cfg(not(feature = "profitdll-dyn"))]
#[link(name = "ProfitDLL")]
unsafe extern "system" {
    // Initialization
    pub fn DLLInitializeLogin(
        pwcActivationKey: PWideChar,
        pwcUser: PWideChar,
        pwcPassword: PWideChar,
        StateCallback: TStateCallback,
        HistoryCallback: THistoryCallback,
        OrderChangeCallback: TOrderChangeCallback,
        AccountCallback: TAccountCallback,
        NewTradeCallback: TNewTradeCallback,
        NewDailyCallback: TNewDailyCallback,
        PriceBookCallback: TPriceBookCallback,
        OfferBookCallback: TOfferBookCallback,
        HistoryTradeCallback: THistoryTradeCallback,
        ProgressCallback: TProgressCallback,
        TinyBookCallback: TTinyBookCallback,
    ) -> i32;

    pub fn DLLInitializeMarketLogin(
        pwcActivationKey: PWideChar,
        pwcUser: PWideChar,
        pwcPassword: PWideChar,
        StateCallback: TStateCallback,
        NewTradeCallback: TNewTradeCallback,
        NewDailyCallback: TNewDailyCallback,
        PriceBookCallback: TPriceBookCallback,
        OfferBookCallback: TOfferBookCallback,
        HistoryTradeCallback: THistoryTradeCallback,
        ProgressCallback: TProgressCallback,
        TinyBookCallback: TTinyBookCallback,
    ) -> i32;

    pub fn DLLFinalize() -> i32;

    // Subscriptions
    pub fn SubscribeTicker(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn UnsubscribeTicker(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn SubscribePriceBook(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn UnsubscribePriceBook(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn SubscribeOfferBook(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn UnsubscribeOfferBook(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;

    // Agents
    pub fn GetAgentNameByID(nID: i32) -> PWideChar;
    pub fn GetAgentShortNameByID(nID: i32) -> PWideChar;
    pub fn GetAgentNameLength(nAgentID: i32, nShortName: u32) -> i32;
    pub fn GetAgentName(nCount: i32, nAgentID: i32, pwcAgent: PWStrMut, nShortName: u32) -> i32;

    // Routing-only legacy
    pub fn GetAccount() -> i32;

    pub fn SendBuyOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        dPrice: f64,
        nAmount: i32,
    ) -> i64;
    pub fn SendSellOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        dPrice: f64,
        nAmount: i32,
    ) -> i64;
    pub fn SendMarketBuyOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        nAmount: i32,
    ) -> i64;
    pub fn SendMarketSellOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        nAmount: i32,
    ) -> i64;
    pub fn SendStopBuyOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        dPrice: f64,
        dStopPrice: f64,
        nAmount: i32,
    ) -> i64;
    pub fn SendStopSellOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        dPrice: f64,
        dStopPrice: f64,
        nAmount: i32,
    ) -> i64;
    pub fn SendChangeOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcstrClOrdID: PWideChar,
        dPrice: f64,
        nAmount: i32,
    ) -> i32;
    pub fn SendCancelOrder(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcClOrdId: PWideChar,
        pwcSenha: PWideChar,
    ) -> i32;
    pub fn SendCancelOrders(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
    ) -> i32;
    pub fn SendCancelAllOrders(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcSenha: PWideChar,
    ) -> i32;
    pub fn SendZeroPosition(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        pwcSenha: PWideChar,
        dPrice: f64,
    ) -> i64;
    pub fn SendZeroPositionAtMarket(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        pwcSenha: PWideChar,
    ) -> i64;

    pub fn GetOrders(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        dtStart: PWideChar,
        dtEnd: PWideChar,
    ) -> i32;
    pub fn GetOrder(pwcClOrdId: PWideChar) -> i32;
    pub fn GetOrderProfitID(nProfitID: i64) -> i32;

    pub fn GetPosition(
        pwcIDAccount: PWideChar,
        pwcIDCorretora: PWideChar,
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
    ) -> *mut core::ffi::c_void;

    // History trades
    pub fn GetHistoryTrades(
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        dtDateStart: PWideChar,
        dtDateEnd: PWideChar,
    ) -> i32;

    // V2 APIs
    pub fn SendOrder(a_SendOrder: PConnectorSendOrder) -> i64;
    pub fn SendChangeOrderV2(a_ChangeOrder: PConnectorChangeOrder) -> i32;
    pub fn SendCancelOrderV2(a_CancelOrder: PConnectorCancelOrder) -> i32;
    pub fn SendCancelOrdersV2(a_CancelOrders: PConnectorCancelOrders) -> i32;
    pub fn SendCancelAllOrdersV2(a_CancelOrder: PConnectorCancelAllOrders) -> i32;
    pub fn SendZeroPositionV2(a_ZeroPosition: PConnectorZeroPosition) -> i64;

    // Accounts & positions
    pub fn GetAccountCount() -> i32;
    pub fn GetAccounts(
        a_nStartSource: i32,
        a_nStartDest: i32,
        a_nCount: i32,
        a_arAccounts: PConnectorAccountIdentifierArrayOut,
    ) -> i32;
    pub fn GetAccountCountByBroker(a_nBrokerID: i32) -> i32;
    pub fn GetAccountsByBroker(
        a_nBrokerID: i32,
        a_nStartSource: i32,
        a_nStartDest: i32,
        a_nCount: i32,
        a_arAccounts: PConnectorAccountIdentifierArrayOut,
    ) -> i32;
    pub fn GetAccountDetails(a_Account: PConnectorTradingAccountOut) -> i32;
    pub fn GetSubAccountCount(a_MasterAccountID: PConnectorAccountIdentifier) -> i32;
    pub fn GetSubAccounts(
        a_MasterAccountID: PConnectorAccountIdentifier,
        a_nStartSource: i32,
        a_nStartDest: i32,
        a_nCount: i32,
        a_arAccounts: PConnectorAccountIdentifierArrayOut,
    ) -> i32;
    pub fn GetPositionV2(a_Position: PConnectorTradingAccountPosition) -> i32;
    pub fn GetOrderDetails(a_Order: PConnectorOrderOut) -> i32;

    // Orders history iteration
    pub fn HasOrdersInInterval(
        a_AccountID: PConnectorAccountIdentifier,
        a_dtStart: TSystemTime,
        a_dtEnd: TSystemTime,
    ) -> NResult;
    pub fn EnumerateOrdersByInterval(
        a_AccountID: PConnectorAccountIdentifier,
        a_OrderVersion: u8,
        a_dtStart: TSystemTime,
        a_dtEnd: TSystemTime,
        a_Param: LPARAM,
        a_Callback: TConnectorEnumerateOrdersProc,
    ) -> NResult;
    pub fn EnumerateAllOrders(
        a_AccountID: PConnectorAccountIdentifier,
        a_OrderVersion: u8,
        a_Param: LPARAM,
        a_Callback: TConnectorEnumerateOrdersProc,
    ) -> NResult;
    pub fn EnumerateAllPositionAssets(
        a_AccountID: PConnectorAccountIdentifier,
        a_AssetVersion: u8,
        a_Param: LPARAM,
        a_Callback: TConnectorEnumerateAssetProc,
    ) -> NResult;

    // Market data server and time
    pub fn SetServerAndPort(strServer: PWideChar, strPort: PWideChar) -> i32;
    pub fn GetServerClock(
        dtDate: *mut f64,
        nYear: *mut i32,
        nMonth: *mut i32,
        nDay: *mut i32,
        nHour: *mut i32,
        nMin: *mut i32,
        nSec: *mut i32,
        nMilisec: *mut i32,
    ) -> i32;

    // Settings & callbacks setters
    pub fn SetDayTrade(bUseDayTrade: i32) -> i32;
    pub fn SetEnabledHistOrder(bEnabled: i32) -> i32;
    pub fn SetEnabledLogToDebug(bEnabled: i32) -> i32;
    pub fn RequestTickerInfo(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn SubscribeAdjustHistory(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn UnsubscribeAdjustHistory(pwcTicker: PWideChar, pwcBolsa: PWideChar) -> i32;
    pub fn GetLastDailyClose(
        pwcTicker: PWideChar,
        pwcBolsa: PWideChar,
        dClose: *mut f64,
        bAdjusted: i32,
    ) -> i32;

    pub fn SetStateCallback(a_StateCallback: TStateCallback) -> i32;
    pub fn SetAssetListCallback(a_AssetListCallback: TAssetListCallback) -> i32;
    pub fn SetAssetListInfoCallback(a_AssetListInfoCallback: TAssetListInfoCallback) -> i32;
    pub fn SetAssetListInfoCallbackV2(a_AssetListInfoCallbackV2: TAssetListInfoCallbackV2) -> i32;
    pub fn SetInvalidTickerCallback(a_InvalidTickerCallback: TInvalidTickerCallback) -> i32;
    pub fn SetTradeCallback(a_TradeCallback: TNewTradeCallback) -> i32; // deprecated in favor of V2
    pub fn SetHistoryTradeCallback(a_HistoryTradeCallback: THistoryTradeCallback) -> i32; // deprecated in favor of V2
    pub fn SetDailyCallback(a_DailyCallback: TNewDailyCallback) -> i32;
    pub fn SetTheoreticalPriceCallback(
        a_TheoreticalPriceCallback: TTheoreticalPriceCallback,
    ) -> i32;
    pub fn SetTinyBookCallback(a_TinyBookCallback: TTinyBookCallback) -> i32;
    pub fn SetChangeCotationCallback(a_ChangeCotation: TChangeCotation) -> i32;
    pub fn SetChangeStateTickerCallback(a_ChangeStateTicker: TChangeStateTicker) -> i32;
    pub fn SetSerieProgressCallback(a_SerieProgressCallback: TProgressCallback) -> i32;
    pub fn SetOfferBookCallback(a_OfferBookCallback: TOfferBookCallback) -> i32;
    pub fn SetOfferBookCallbackV2(a_OfferBookCallbackV2: TOfferBookCallbackV2) -> i32;
    pub fn SetPriceBookCallback(a_PriceBookCallback: TPriceBookCallback) -> i32;
    pub fn SetPriceBookCallbackV2(a_PriceBookCallbackV2: TPriceBookCallbackV2) -> i32;
    pub fn SetAdjustHistoryCallback(a_AdjustHistoryCallback: TAdjustHistoryCallback) -> i32;
    pub fn SetAdjustHistoryCallbackV2(a_AdjustHistoryCallbackV2: TAdjustHistoryCallbackV2) -> i32;
    pub fn SetAssetPositionListCallback(
        a_AssetPositionListCallback: TConnectorAssetPositionListCallback,
    ) -> i32;
    pub fn SetAccountCallback(a_AccountCallback: TAccountCallback) -> i32;
    pub fn SetHistoryCallback(a_HistoryCallback: THistoryCallback) -> i32; // deprecated
    pub fn SetHistoryCallbackV2(a_HistoryCallbackV2: THistoryCallbackV2) -> i32; // deprecated in favor of SetOrderHistoryCallback
    pub fn SetOrderChangeCallback(a_OrderChangeCallback: TOrderChangeCallback) -> i32; // deprecated
    pub fn SetOrderChangeCallbackV2(a_OrderChangeCallbackV2: TOrderChangeCallbackV2) -> i32; // deprecated
    pub fn SetOrderCallback(a_OrderCallback: TConnectorOrderCallback) -> i32;
    pub fn SetOrderHistoryCallback(a_OrderHistoryCallback: TConnectorAccountCallback) -> NResult;
    pub fn SetTradeCallbackV2(a_TradeCallbackV2: TConnectorTradeCallback) -> NResult;
    pub fn SetHistoryTradeCallbackV2(a_HistoryTradeCallbackV2: TConnectorTradeCallback) -> NResult;

    // Utilities
    pub fn TranslateTrade(a_pTrade: *const core::ffi::c_void, a_Trade: PConnectorTrade) -> NResult;
    // Free DLL-allocated buffers (arrays returned in callbacks)
    pub fn FreePointer(pointer: *mut core::ffi::c_void, nSize: i32) -> i32;
}
