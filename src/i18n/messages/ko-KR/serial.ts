// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "serial.errors.missingConfiguration": "시리얼 설정이 없습니다",
  "serial.connect.title": "시리얼 연결",
  "serial.form.namePlaceholder": "예: 개발 보드",
  "serial.form.stopBits": "정지 비트",
  "serial.parity.even": "짝수",
  "serial.actions.refresh": "시리얼 포트 새로 고침",
  "serial.parity.odd": "홀수",
  "serial.form.encoding": "문자 인코딩",
  "serial.form.tagsPlaceholder": "보드, 디버깅",
  "serial.actions.open": "시리얼 포트 여는 중",
  "serial.form.dataBits": "데이터 비트",
  "serial.actions.newHost": "새 시리얼 호스트",
  "serial.empty":
    "시리얼 포트를 찾을 수 없습니다. 기기를 연결한 후 새로 고침하거나 이름을 직접 입력하세요. Linux에서는 시리얼 포트 권한도 확인하세요.",
  "serial.form.parity": "패리티",
  "serial.form.baudRate": "전송 속도",
  "serial.validation.baudRate":
    "전송 속도는 1~4,000,000 사이의 정수여야 합니다",
  "serial.form.flowControl": "흐름 제어",
  "serial.actions.editHost": "시리얼 호스트 편집",
  "serial.validation.device":
    "제어 문자 없이 1,024바이트 이하의 시리얼 기기 경로 또는 이름을 입력하세요",
  "serial.form.devicePlaceholder": "시리얼 기기 경로 또는 이름 선택 또는 입력",
  "serial.form.baudRatePlaceholder": "전송 속도 선택 또는 입력(기본값 9600)",
  "serial.actions.reopen":
    "시리얼 포트를 다시 여는 중입니다. 터미널 기록은 보존됩니다.",
  "serial.form.advanced": "고급 설정",
} as const;
