//! Transactional native Unicode clipboard publication for local speech transcription.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClipboardFailure {
    StaUnavailable,
    QueueFull,
    InvalidText,
    Timeout,
    PublishRejected,
    PasteTargetUnavailable,
    PasteTargetChanged,
    PasteFocusDenied,
    PasteInputRejected,
}

impl ClipboardFailure {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::StaUnavailable => "clipboard-sta-unavailable",
            Self::QueueFull => "clipboard-queue-full",
            Self::InvalidText => "clipboard-invalid-text",
            Self::Timeout => "clipboard-timeout",
            Self::PublishRejected => "clipboard-publish-rejected",
            Self::PasteTargetUnavailable => "paste-target-unavailable",
            Self::PasteTargetChanged => "paste-target-changed",
            Self::PasteFocusDenied => "paste-focus-denied",
            Self::PasteInputRejected => "paste-input-rejected",
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_clipboard {
    use super::ClipboardFailure;
    use std::mem::ManuallyDrop;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::{mpsc, Arc, OnceLock};
    use std::time::{Duration, Instant};

    use windows::core::{implement, Error, HRESULT};
    use windows::Win32::Foundation::{
        CLIPBRD_E_CANT_OPEN, DATA_S_SAMEFORMATETC, DV_E_DVASPECT, DV_E_FORMATETC, DV_E_LINDEX,
        DV_E_TYMED, E_NOTIMPL, OLE_E_ADVISENOTSUPPORTED, RPC_E_CALL_REJECTED,
        RPC_E_SERVERCALL_RETRYLATER,
    };
    use windows::Win32::System::Com::{
        IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA, DATADIR_GET,
        DATADIR_SET, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::{
        OleFlushClipboard, OleInitialize, OleSetClipboard, OleUninitialize, CF_UNICODETEXT,
    };
    use windows::Win32::UI::Shell::SHCreateStdEnumFmtEtc;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };
    use windows_core::{Ref, BOOL};

    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    const PUMP_INTERVAL: Duration = Duration::from_millis(10);
    const MAX_SET_ATTEMPTS: usize = 3;
    const PENDING: u8 = 0;
    const PUBLISHING: u8 = 1;
    const CANCELLED: u8 = 2;
    const COMPLETE: u8 = 3;

    unsafe extern "system" {
        fn GlobalFree(
            memory: windows::Win32::Foundation::HGLOBAL,
        ) -> windows::Win32::Foundation::HGLOBAL;
    }

    fn error(code: HRESULT) -> Error {
        Error::from_hresult(code)
    }

    fn supports_unicode_text(format: *const FORMATETC) -> Result<(), HRESULT> {
        let Some(format) = (unsafe { format.as_ref() }) else {
            return Err(DV_E_FORMATETC);
        };
        if format.cfFormat != CF_UNICODETEXT.0 {
            return Err(DV_E_FORMATETC);
        }
        if format.dwAspect != DVASPECT_CONTENT.0 as u32 {
            return Err(DV_E_DVASPECT);
        }
        if format.lindex != -1 {
            return Err(DV_E_LINDEX);
        }
        if format.tymed & TYMED_HGLOBAL.0 as u32 == 0 {
            return Err(DV_E_TYMED);
        }
        Ok(())
    }

    fn unicode_text_format() -> FORMATETC {
        FORMATETC {
            cfFormat: CF_UNICODETEXT.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0 as u32,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        }
    }

    fn allocate_text(utf16: &[u16]) -> windows::core::Result<windows::Win32::Foundation::HGLOBAL> {
        unsafe {
            let memory = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of_val(utf16))?;
            let pointer = GlobalLock(memory);
            if pointer.is_null() {
                let _ = GlobalFree(memory);
                return Err(Error::from_win32());
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), pointer.cast::<u16>(), utf16.len());
            // Zero means either success or failure; GetLastError is unchanged on successful unlock.
            let _ = GlobalUnlock(memory);
            Ok(memory)
        }
    }

    #[implement(IDataObject)]
    struct UnicodeClipboardDataObject {
        // Includes one terminal NUL and contains no earlier NULs.
        utf16: Box<[u16]>,
    }

    impl UnicodeClipboardDataObject {
        fn new(text: &str) -> Result<Self, ()> {
            if text.contains('\0') {
                return Err(());
            }
            let mut utf16 = Vec::with_capacity(text.encode_utf16().count() + 1);
            utf16.extend(text.encode_utf16());
            utf16.push(0);
            Ok(Self {
                utf16: utf16.into_boxed_slice(),
            })
        }
    }

    impl IDataObject_Impl for UnicodeClipboardDataObject_Impl {
        fn GetData(&self, format: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
            supports_unicode_text(format).map_err(error)?;
            let memory = allocate_text(&self.utf16)?;
            Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: memory },
                // COM owns and releases hGlobal when this field is null.
                pUnkForRelease: ManuallyDrop::new(None),
            })
        }

        fn GetDataHere(
            &self,
            _format: *const FORMATETC,
            _medium: *mut STGMEDIUM,
        ) -> windows::core::Result<()> {
            Err(error(E_NOTIMPL))
        }

        fn QueryGetData(&self, format: *const FORMATETC) -> HRESULT {
            supports_unicode_text(format).map_or_else(|code| code, |()| HRESULT(0))
        }

        fn GetCanonicalFormatEtc(
            &self,
            _input: *const FORMATETC,
            output: *mut FORMATETC,
        ) -> HRESULT {
            if let Some(output) = unsafe { output.as_mut() } {
                output.ptd = std::ptr::null_mut();
            }
            DATA_S_SAMEFORMATETC
        }

        fn SetData(
            &self,
            _format: *const FORMATETC,
            _medium: *const STGMEDIUM,
            _release: BOOL,
        ) -> windows::core::Result<()> {
            Err(error(E_NOTIMPL))
        }

        fn EnumFormatEtc(&self, direction: u32) -> windows::core::Result<IEnumFORMATETC> {
            if direction == DATADIR_GET.0 as u32 {
                unsafe { SHCreateStdEnumFmtEtc(&[unicode_text_format()]) }
            } else if direction == DATADIR_SET.0 as u32 {
                Err(error(E_NOTIMPL))
            } else {
                Err(error(DV_E_FORMATETC))
            }
        }

        fn DAdvise(
            &self,
            _format: *const FORMATETC,
            _flags: u32,
            _sink: Ref<'_, IAdviseSink>,
        ) -> windows::core::Result<u32> {
            Err(error(OLE_E_ADVISENOTSUPPORTED))
        }

        fn DUnadvise(&self, _connection: u32) -> windows::core::Result<()> {
            Err(error(OLE_E_ADVISENOTSUPPORTED))
        }

        fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> {
            Err(error(OLE_E_ADVISENOTSUPPORTED))
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SetOutcome {
        Committed,
        TransientRejection,
        PermanentRejection,
    }

    trait ClipboardPublisher {
        fn set(&self, data: &IDataObject) -> SetOutcome;
        fn flush(&self) -> Result<(), ()>;
    }

    struct OleClipboardPublisher;

    impl ClipboardPublisher for OleClipboardPublisher {
        fn set(&self, data: &IDataObject) -> SetOutcome {
            match unsafe { OleSetClipboard(data) } {
                Ok(()) => SetOutcome::Committed,
                Err(error)
                    if matches!(
                        error.code(),
                        CLIPBRD_E_CANT_OPEN | RPC_E_CALL_REJECTED | RPC_E_SERVERCALL_RETRYLATER
                    ) =>
                {
                    SetOutcome::TransientRejection
                }
                Err(_) => SetOutcome::PermanentRejection,
            }
        }

        fn flush(&self) -> Result<(), ()> {
            unsafe { OleFlushClipboard().map_err(|_| ()) }
        }
    }

    struct OleApartment;

    impl OleApartment {
        fn initialize_sta() -> Result<Self, ()> {
            unsafe { OleInitialize(None).map_err(|_| ())? };
            Ok(Self)
        }
    }

    impl Drop for OleApartment {
        fn drop(&mut self) {
            unsafe { OleUninitialize() };
        }
    }

    fn set_with_retry(
        data: &IDataObject,
        publisher: &impl ClipboardPublisher,
    ) -> Result<(), ClipboardFailure> {
        for attempt in 0..MAX_SET_ATTEMPTS {
            match publisher.set(data) {
                SetOutcome::Committed => return Ok(()),
                SetOutcome::PermanentRejection => break,
                SetOutcome::TransientRejection if attempt + 1 < MAX_SET_ATTEMPTS => {
                    std::thread::sleep(PUMP_INTERVAL);
                }
                SetOutcome::TransientRejection => break,
            }
        }
        Err(ClipboardFailure::PublishRejected)
    }

    fn publish_with(
        text: &str,
        publisher: &impl ClipboardPublisher,
    ) -> Result<(), ClipboardFailure> {
        let object: IDataObject = UnicodeClipboardDataObject::new(text)
            .map_err(|_| ClipboardFailure::InvalidText)?
            .into();
        set_with_retry(&object, publisher)?;
        // Set commits ownership. Flush improves durability, but failure cannot safely be reported
        // as rejection because the new delayed-rendering owner is already observable.
        let _ = publisher.flush();
        Ok(())
    }

    struct Request {
        text: String,
        deadline: Instant,
        state: Arc<AtomicU8>,
        reply: mpsc::SyncSender<Result<(), ClipboardFailure>>,
    }

    static WORKER: OnceLock<Result<mpsc::SyncSender<Request>, ClipboardFailure>> = OnceLock::new();

    fn pump_messages() {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn start_worker() -> Result<mpsc::SyncSender<Request>, ClipboardFailure> {
        let (request_tx, request_rx) = mpsc::sync_channel::<Request>(1);
        std::thread::Builder::new()
            .name("speech-clipboard-sta".into())
            .spawn(move || {
                let Ok(_apartment) = OleApartment::initialize_sta() else {
                    while let Ok(request) = request_rx.recv() {
                        request.state.store(COMPLETE, Ordering::Release);
                        let _ = request.reply.send(Err(ClipboardFailure::StaUnavailable));
                    }
                    return;
                };
                // Retain the current source object in its creating apartment. OLE may request
                // delayed data from it for as long as this app-lifetime worker is alive.
                let mut current_owner: Option<IDataObject> = None;
                loop {
                    pump_messages();
                    let request = match request_rx.recv_timeout(PUMP_INTERVAL) {
                        Ok(request) => request,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if Instant::now() >= request.deadline
                        || request
                            .state
                            .compare_exchange(
                                PENDING,
                                PUBLISHING,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            )
                            .is_err()
                    {
                        request.state.store(COMPLETE, Ordering::Release);
                        let _ = request.reply.send(Err(ClipboardFailure::Timeout));
                        continue;
                    }
                    let result = UnicodeClipboardDataObject::new(&request.text)
                        .map_err(|_| ClipboardFailure::InvalidText)
                        .and_then(|value| {
                            let object: IDataObject = value.into();
                            set_with_retry(&object, &OleClipboardPublisher)?;
                            current_owner = Some(object);
                            pump_messages();
                            // Post-set failure remains committed success; retained owner continues
                            // serving delayed CF_UNICODETEXT for the app lifetime.
                            let _ = OleClipboardPublisher.flush();
                            Ok(())
                        });
                    request.state.store(COMPLETE, Ordering::Release);
                    let _ = request.reply.send(result);
                    // Service IDataObject calls generated during publication before next request.
                    pump_messages();
                    let _ = &current_owner;
                }
            })
            .map_err(|_| ClipboardFailure::StaUnavailable)?;
        Ok(request_tx)
    }

    fn enqueue_and_wait(
        worker: &mpsc::SyncSender<Request>,
        text: &str,
        timeout: Duration,
    ) -> Result<(), ClipboardFailure> {
        let state = Arc::new(AtomicU8::new(PENDING));
        let (reply, result) = mpsc::sync_channel(1);
        worker
            .try_send(Request {
                text: text.to_owned(),
                deadline: Instant::now() + timeout,
                state: Arc::clone(&state),
                reply,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => ClipboardFailure::QueueFull,
                mpsc::TrySendError::Disconnected(_) => ClipboardFailure::StaUnavailable,
            })?;
        match result.recv_timeout(timeout) {
            Ok(value) => value,
            Err(_) => {
                // Returning failure is safe only when cancellation wins while still pending.
                // Once publication starts, wait without another timeout for its exact result.
                if state
                    .compare_exchange(PENDING, CANCELLED, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    Err(ClipboardFailure::Timeout)
                } else {
                    result
                        .recv()
                        .map_err(|_| ClipboardFailure::StaUnavailable)?
                }
            }
        }
    }

    pub(crate) fn write_unicode_text(text: &str) -> Result<(), ClipboardFailure> {
        let worker = WORKER
            .get_or_init(start_worker)
            .as_ref()
            .map_err(|error| *error)?;
        enqueue_and_wait(worker, text, REQUEST_TIMEOUT)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;
        use std::collections::VecDeque;
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::time::Duration;

        fn restore_with<E>(
            set: impl FnOnce() -> Result<(), E>,
            flush: impl FnOnce() -> Result<(), E>,
        ) -> Result<(), E> {
            set()?;
            flush()
        }

        #[test]
        fn explicit_restore_reports_set_and_flush_failures() {
            assert_eq!(restore_with(|| Err("set"), || Ok(())), Err("set"));
            assert_eq!(restore_with(|| Ok(()), || Err("flush")), Err("flush"));
            assert_eq!(restore_with(|| Ok::<_, &str>(()), || Ok(())), Ok(()));
        }

        struct RecordingPublisher {
            outcomes: Mutex<VecDeque<SetOutcome>>,
            events: Mutex<Vec<&'static str>>,
        }

        impl RecordingPublisher {
            fn new(outcomes: impl IntoIterator<Item = SetOutcome>) -> Self {
                Self {
                    outcomes: Mutex::new(outcomes.into_iter().collect()),
                    events: Mutex::new(Vec::new()),
                }
            }

            fn events(&self) -> Vec<&'static str> {
                self.events.lock().expect("events lock").clone()
            }
        }

        impl ClipboardPublisher for RecordingPublisher {
            fn set(&self, _data: &IDataObject) -> SetOutcome {
                self.events.lock().expect("events lock").push("set");
                self.outcomes
                    .lock()
                    .expect("outcomes lock")
                    .pop_front()
                    .expect("configured set outcome")
            }

            fn flush(&self) -> Result<(), ()> {
                self.events.lock().expect("events lock").push("flush");
                Ok(())
            }
        }

        #[test]
        fn format_enumerator_advertises_exact_unicode_hglobal_shape() {
            let object: IDataObject = UnicodeClipboardDataObject::new("sentinel")
                .expect("valid text")
                .into();
            let enumerator = unsafe {
                object
                    .EnumFormatEtc(DATADIR_GET.0 as u32)
                    .expect("GET enumeration")
            };
            let mut formats = [FORMATETC::default(); 2];
            let mut fetched = 0;

            let status = unsafe { enumerator.Next(&mut formats, Some(&mut fetched)) };

            assert_eq!(fetched, 1);
            assert_ne!(status, HRESULT(0));
            assert_eq!(formats[0].cfFormat, CF_UNICODETEXT.0);
            assert_eq!(formats[0].dwAspect, DVASPECT_CONTENT.0 as u32);
            assert_eq!(formats[0].lindex, -1);
            assert_eq!(formats[0].tymed, TYMED_HGLOBAL.0 as u32);
            assert!(formats[0].ptd.is_null());
            assert!(unsafe { object.EnumFormatEtc(DATADIR_SET.0 as u32) }.is_err());
        }

        #[test]
        fn incompatible_format_shapes_are_rejected() {
            let mut format = unicode_text_format();
            format.dwAspect = 0;
            assert_eq!(supports_unicode_text(&format), Err(DV_E_DVASPECT));
            format = unicode_text_format();
            format.lindex = 0;
            assert_eq!(supports_unicode_text(&format), Err(DV_E_LINDEX));
        }

        #[test]
        fn publish_retries_transient_rejection_then_flushes_after_set() {
            let publisher =
                RecordingPublisher::new([SetOutcome::TransientRejection, SetOutcome::Committed]);

            assert_eq!(publish_with("sentinel", &publisher), Ok(()));
            assert_eq!(publisher.events(), vec!["set", "set", "flush"]);
        }

        #[test]
        fn failed_set_never_flushes() {
            let publisher = RecordingPublisher::new([SetOutcome::PermanentRejection]);

            assert_eq!(
                publish_with("sentinel", &publisher),
                Err(ClipboardFailure::PublishRejected)
            );
            assert_eq!(publisher.events(), vec!["set"]);
        }

        #[test]
        fn transient_set_retries_are_bounded_and_never_flush() {
            let publisher = RecordingPublisher::new([
                SetOutcome::TransientRejection,
                SetOutcome::TransientRejection,
                SetOutcome::TransientRejection,
            ]);

            assert_eq!(
                publish_with("sentinel", &publisher),
                Err(ClipboardFailure::PublishRejected)
            );
            assert_eq!(publisher.events(), vec!["set", "set", "set"]);
        }

        #[test]
        fn flush_failure_after_committed_set_is_success() {
            struct FlushFailingPublisher;
            impl ClipboardPublisher for FlushFailingPublisher {
                fn set(&self, _data: &IDataObject) -> SetOutcome {
                    SetOutcome::Committed
                }

                fn flush(&self) -> Result<(), ()> {
                    Err(())
                }
            }

            assert_eq!(publish_with("sentinel", &FlushFailingPublisher), Ok(()));
        }

        #[test]
        #[ignore = "writes OS clipboard; requires JASONSHELL_PHYSICAL_CLIPBOARD_TEST=1"]
        fn physical_unicode_round_trip_is_visible_through_standard_clipboard_api() {
            use windows::Win32::Foundation::HGLOBAL;
            use windows::Win32::System::DataExchange::{
                CloseClipboard, GetClipboardData, OpenClipboard,
            };
            use windows::Win32::System::Ole::OleGetClipboard;

            assert_eq!(
                std::env::var("JASONSHELL_PHYSICAL_CLIPBOARD_TEST").as_deref(),
                Ok("1"),
                "explicit JASONSHELL_PHYSICAL_CLIPBOARD_TEST=1 authorization required"
            );

            struct ClipboardRestoreGuard {
                prior: Option<IDataObject>,
            }
            impl ClipboardRestoreGuard {
                fn restore(&mut self) -> windows::core::Result<()> {
                    let prior = self
                        .prior
                        .as_ref()
                        .expect("clipboard restore is single-use");
                    restore_with(
                        || unsafe { OleSetClipboard(prior) },
                        || unsafe { OleFlushClipboard() },
                    )?;
                    self.prior = None;
                    Ok(())
                }
            }
            impl Drop for ClipboardRestoreGuard {
                fn drop(&mut self) {
                    if let Some(prior) = self.prior.as_ref() {
                        unsafe {
                            let _ = OleSetClipboard(prior);
                            let _ = OleFlushClipboard();
                        }
                    }
                }
            }
            struct OpenClipboardGuard;
            impl Drop for OpenClipboardGuard {
                fn drop(&mut self) {
                    unsafe {
                        let _ = CloseClipboard();
                    }
                }
            }

            let _apartment = OleApartment::initialize_sta().expect("initialize test STA");
            let mut restore = ClipboardRestoreGuard {
                prior: Some(unsafe { OleGetClipboard().expect("capture prior OLE clipboard") }),
            };
            let sentinel = format!(
                "jasonshell-clipboard-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time after epoch")
                    .as_nanos()
            );
            let object: IDataObject = UnicodeClipboardDataObject::new(&sentinel)
                .expect("valid sentinel")
                .into();
            assert!(matches!(
                OleClipboardPublisher.set(&object),
                SetOutcome::Committed
            ));
            let retained_owner = object;
            pump_messages();
            OleClipboardPublisher
                .flush()
                .expect("materialize clipboard");

            unsafe { OpenClipboard(None).expect("open standard clipboard") };
            let open = OpenClipboardGuard;
            let handle =
                unsafe { GetClipboardData(CF_UNICODETEXT.0 as u32).expect("read CF_UNICODETEXT") };
            let pointer = unsafe { GlobalLock(HGLOBAL(handle.0)) }.cast::<u16>();
            assert!(!pointer.is_null(), "lock CF_UNICODETEXT");
            let mut length = 0usize;
            while unsafe { *pointer.add(length) } != 0 {
                length += 1;
            }
            let actual = String::from_utf16(unsafe { std::slice::from_raw_parts(pointer, length) })
                .expect("valid UTF-16 clipboard text");
            let _ = unsafe { GlobalUnlock(HGLOBAL(handle.0)) };

            assert_eq!(actual, sentinel);
            drop(open);
            drop(retained_owner);
            restore
                .restore()
                .expect("restore and materialize prior clipboard");
        }

        struct FailingPublisher<'a> {
            sentinel: &'a Cell<&'static str>,
        }

        impl ClipboardPublisher for FailingPublisher<'_> {
            fn set(&self, _data: &IDataObject) -> SetOutcome {
                // Models failed transactional publication: existing clipboard owner stays intact.
                assert_eq!(self.sentinel.get(), "existing clipboard");
                SetOutcome::PermanentRejection
            }

            fn flush(&self) -> Result<(), ()> {
                panic!("failed set must not flush")
            }
        }

        #[test]
        fn failed_publication_leaves_existing_clipboard_sentinel_untouched() {
            let sentinel = Cell::new("existing clipboard");

            let result = publish_with(
                "new transcript",
                &FailingPublisher {
                    sentinel: &sentinel,
                },
            );

            assert_eq!(result, Err(ClipboardFailure::PublishRejected));
            assert_eq!(sentinel.get(), "existing clipboard");
        }

        #[test]
        fn embedded_nul_is_rejected_before_publication() {
            struct MustNotPublish;
            impl ClipboardPublisher for MustNotPublish {
                fn set(&self, _data: &IDataObject) -> SetOutcome {
                    panic!("invalid payload reached publisher")
                }

                fn flush(&self) -> Result<(), ()> {
                    panic!("invalid payload reached publisher")
                }
            }

            assert_eq!(
                publish_with("before\0after", &MustNotPublish),
                Err(ClipboardFailure::InvalidText)
            );
        }

        #[test]
        fn cancelled_request_cannot_enter_publication() {
            let state = Arc::new(AtomicU8::new(PENDING));
            assert!(state
                .compare_exchange(PENDING, CANCELLED, Ordering::AcqRel, Ordering::Acquire)
                .is_ok());
            assert!(state
                .compare_exchange(PENDING, PUBLISHING, Ordering::AcqRel, Ordering::Acquire)
                .is_err());
        }

        #[test]
        fn publishing_request_waits_beyond_second_timeout_for_definitive_result() {
            let (worker, requests) = mpsc::sync_channel(1);
            let operation_duration = REQUEST_TIMEOUT + REQUEST_TIMEOUT + Duration::from_millis(100);
            let worker_thread = std::thread::spawn(move || {
                let request: Request = requests.recv().expect("request");
                request
                    .state
                    .compare_exchange(PENDING, PUBLISHING, Ordering::AcqRel, Ordering::Acquire)
                    .expect("publication starts");
                std::thread::sleep(operation_duration);
                request.state.store(COMPLETE, Ordering::Release);
                request.reply.send(Ok(())).expect("caller waits for reply");
            });
            let started = Instant::now();

            let result = enqueue_and_wait(&worker, "transcript", REQUEST_TIMEOUT);

            assert_eq!(result, Ok(()));
            assert!(started.elapsed() >= operation_duration);
            worker_thread.join().expect("worker exits");
        }

        #[test]
        fn full_stalled_queue_returns_before_worker_receives_request() {
            let (worker, requests) = mpsc::sync_channel(1);
            let blocker_state = Arc::new(AtomicU8::new(PENDING));
            let (blocker_reply, _blocker_result) = mpsc::sync_channel(1);
            worker
                .send(Request {
                    text: "blocker".into(),
                    deadline: Instant::now() + REQUEST_TIMEOUT,
                    state: blocker_state,
                    reply: blocker_reply,
                })
                .expect("fill queue");
            let started = Instant::now();

            let result = enqueue_and_wait(&worker, "transcript", REQUEST_TIMEOUT);

            assert_eq!(result, Err(ClipboardFailure::QueueFull));
            assert!(started.elapsed() < Duration::from_millis(100));
            let queued = requests.try_recv().expect("only blocker queued");
            assert_eq!(queued.text, "blocker");
            assert!(matches!(
                requests.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn write_unicode_text(text: &str) -> Result<(), ClipboardFailure> {
    windows_clipboard::write_unicode_text(text)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn write_unicode_text(_text: &str) -> Result<(), ClipboardFailure> {
    Err(ClipboardFailure::StaUnavailable)
}

#[cfg(test)]
mod classification_tests {
    use super::*;

    #[test]
    fn opaque_failure_codes_are_exact_and_exhaustive() {
        assert_eq!(
            ClipboardFailure::StaUnavailable.code(),
            "clipboard-sta-unavailable"
        );
        assert_eq!(ClipboardFailure::QueueFull.code(), "clipboard-queue-full");
        assert_eq!(
            ClipboardFailure::InvalidText.code(),
            "clipboard-invalid-text"
        );
        assert_eq!(ClipboardFailure::Timeout.code(), "clipboard-timeout");
        assert_eq!(
            ClipboardFailure::PublishRejected.code(),
            "clipboard-publish-rejected"
        );
    }
}
