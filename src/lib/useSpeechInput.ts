import { useCallback, useEffect, useRef, useState } from "react";
import { requestMicrophonePermission, type SpeechInputState } from "./speech";
import { transcribeAudio } from "./tauri";
import { WavRecorder } from "./wavRecorder";

interface UseSpeechInputOptions {
  lang?: string;
  /** Called when a phrase is finalized */
  onFinal: (transcript: string) => void;
  disabled?: boolean;
}

/**
 * Voice input: record mic audio in the WebView, then transcribe via
 * OpenAI-compatible Whisper (`/audio/transcriptions`) on the Rust side.
 *
 * Apple SFSpeechRecognizer is intentionally avoided — under Cursor/VS Code
 * `tauri dev`, TCC aborts the process even with NSSpeechRecognitionUsageDescription.
 */
export function useSpeechInput({
  lang = "zh-CN",
  onFinal,
  disabled = false,
}: UseSpeechInputOptions) {
  const [state, setState] = useState<SpeechInputState>("idle");
  const [interimText, setInterimText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const recorderRef = useRef<WavRecorder | null>(null);
  const onFinalRef = useRef(onFinal);
  onFinalRef.current = onFinal;
  const busyRef = useRef(false);

  const stop = useCallback(async () => {
    if (busyRef.current) return;
    const recorder = recorderRef.current;
    if (!recorder) {
      setState("idle");
      setInterimText("");
      return;
    }

    busyRef.current = true;
    setInterimText("正在识别…");
    setState("requesting");

    try {
      const { base64, durationMs } = await recorder.stop();
      recorderRef.current = null;

      if (durationMs < 400) {
        setError("说话时间太短，请再说一次");
        setState("error");
        setInterimText("");
        return;
      }

      const text = await transcribeAudio(base64, lang);
      if (text.trim()) {
        onFinalRef.current(text.trim());
      } else {
        setError("未识别到语音内容");
        setState("error");
        setInterimText("");
        return;
      }
      setInterimText("");
      setState("idle");
    } catch (e) {
      recorderRef.current = null;
      setError(e instanceof Error ? e.message : String(e));
      setState("error");
      setInterimText("");
    } finally {
      busyRef.current = false;
    }
  }, [lang]);

  const cancel = useCallback(() => {
    recorderRef.current?.cancel();
    recorderRef.current = null;
    setInterimText("");
    setState("idle");
    busyRef.current = false;
  }, []);

  const start = useCallback(async () => {
    if (disabled || busyRef.current) return;

    setError(null);
    setState("requesting");
    setInterimText("");

    const permitted = await requestMicrophonePermission();
    if (!permitted) {
      setError("无法访问麦克风，请在系统设置中授予权限");
      setState("error");
      return;
    }

    try {
      const recorder = new WavRecorder();
      await recorder.start();
      recorderRef.current = recorder;
      setState("listening");
      setInterimText("正在聆听，再次点击麦克风结束…");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setState("error");
    }
  }, [disabled]);

  const toggle = useCallback(async () => {
    if (state === "listening") {
      await stop();
    } else if (state === "requesting") {
      cancel();
    } else {
      await start();
    }
  }, [cancel, start, state, stop]);

  useEffect(() => {
    return () => {
      recorderRef.current?.cancel();
      recorderRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (disabled && (state === "listening" || state === "requesting")) {
      cancel();
    }
  }, [cancel, disabled, state]);

  return {
    isSupported: typeof navigator !== "undefined" && !!navigator.mediaDevices?.getUserMedia,
    isListening: state === "listening",
    isRequesting: state === "requesting",
    interimText,
    error,
    state,
    start,
    stop,
    toggle,
    clearError: () => {
      setError(null);
      if (state === "error") setState("idle");
    },
  };
}
