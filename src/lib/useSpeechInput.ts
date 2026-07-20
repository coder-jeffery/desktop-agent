import { useCallback, useEffect, useRef, useState } from "react";
import {
  getSpeechRecognitionCtor,
  isSpeechInputSupported,
  mapSpeechError,
  requestMicrophonePermission,
  type SpeechInputState,
} from "./speech";

interface UseSpeechInputOptions {
  lang?: string;
  /** Called when a phrase is finalized */
  onFinal: (transcript: string) => void;
  disabled?: boolean;
}

export function useSpeechInput({
  lang = "zh-CN",
  onFinal,
  disabled = false,
}: UseSpeechInputOptions) {
  const [state, setState] = useState<SpeechInputState>("idle");
  const [interimText, setInterimText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const recognitionRef = useRef<SpeechRecognition | null>(null);
  const onFinalRef = useRef(onFinal);
  onFinalRef.current = onFinal;

  const stop = useCallback(() => {
    recognitionRef.current?.stop();
    recognitionRef.current = null;
    setInterimText("");
    setState("idle");
  }, []);

  const start = useCallback(async () => {
    if (disabled) return;

    const Ctor = getSpeechRecognitionCtor();
    if (!Ctor) {
      setError("当前环境不支持语音输入（需要 Web Speech API）");
      setState("error");
      return;
    }

    setError(null);
    setState("requesting");

    const permitted = await requestMicrophonePermission();
    if (!permitted) {
      setError("无法访问麦克风，请在系统设置中授予权限");
      setState("error");
      return;
    }

    try {
      const recognition = new Ctor();
      recognition.lang = lang;
      recognition.continuous = true;
      recognition.interimResults = true;
      recognition.maxAlternatives = 1;

      recognition.onstart = () => {
        setState("listening");
        setInterimText("");
      };

      recognition.onresult = (event: SpeechRecognitionEvent) => {
        let interim = "";
        let finalChunk = "";

        for (let i = event.resultIndex; i < event.results.length; i++) {
          const result = event.results[i];
          const text = result[0]?.transcript ?? "";
          if (result.isFinal) {
            finalChunk += text;
          } else {
            interim += text;
          }
        }

        setInterimText(interim);
        if (finalChunk.trim()) {
          onFinalRef.current(finalChunk.trim());
          setInterimText("");
        }
      };

      recognition.onerror = (event: SpeechRecognitionErrorEvent) => {
        if (event.error === "aborted") {
          setState("idle");
          return;
        }
        setError(mapSpeechError(event.error));
        setState("error");
        setInterimText("");
      };

      recognition.onend = () => {
        recognitionRef.current = null;
        setInterimText("");
        setState((s) => (s === "listening" ? "idle" : s));
      };

      recognitionRef.current = recognition;
      recognition.start();
    } catch (e) {
      setError(String(e));
      setState("error");
    }
  }, [disabled, lang]);

  const toggle = useCallback(async () => {
    if (state === "listening" || state === "requesting") {
      stop();
    } else {
      await start();
    }
  }, [start, state, stop]);

  useEffect(() => {
    return () => {
      recognitionRef.current?.abort();
      recognitionRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (disabled && (state === "listening" || state === "requesting")) {
      stop();
    }
  }, [disabled, state, stop]);

  return {
    isSupported: isSpeechInputSupported(),
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
