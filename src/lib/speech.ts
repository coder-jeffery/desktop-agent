export type SpeechInputState =
  | "idle"
  | "requesting"
  | "listening"
  | "error";

export interface SpeechInputError {
  code: string;
  message: string;
}

export function getSpeechRecognitionCtor():
  | (new () => SpeechRecognition)
  | null {
  if (typeof window === "undefined") return null;
  const w = window as Window & {
    SpeechRecognition?: new () => SpeechRecognition;
    webkitSpeechRecognition?: new () => SpeechRecognition;
  };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
}

export function isSpeechInputSupported(): boolean {
  return getSpeechRecognitionCtor() !== null;
}

export function mapSpeechError(code: string): string {
  switch (code) {
    case "not-allowed":
    case "service-not-allowed":
      return "麦克风或语音识别权限被拒绝，请在系统设置中允许 Desktop Agent 使用麦克风";
    case "no-speech":
      return "未检测到语音，请再试一次";
    case "audio-capture":
      return "无法访问麦克风，请检查设备连接";
    case "network":
      return "语音识别需要网络连接";
    case "aborted":
      return "语音输入已取消";
    default:
      return `语音识别失败 (${code})`;
  }
}

export async function requestMicrophonePermission(): Promise<boolean> {
  if (!navigator.mediaDevices?.getUserMedia) return true;
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    stream.getTracks().forEach((t) => t.stop());
    return true;
  } catch {
    return false;
  }
}
