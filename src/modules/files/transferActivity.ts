import type { PlainMessageKey } from "@/i18n";
import type { TransferProgress } from "@/services/desktop/types";

export interface SftpTransferActivity {
  id: string;
  label: string;
  path?: string;
  status: "queued" | "running" | "done" | "cancelled" | "failed";
  progress?: TransferProgress;
  message?: string;
  messageKey?: PlainMessageKey;
  cancelling?: boolean;
}

// 批次字节数随当前文件重置，整体进度使用完成文件数；仅单文件按字节计算。
export function transferPercentage(task: SftpTransferActivity): number | null {
  if (task.status === "done") return 100;
  const value = task.progress;
  if (!value) return null;
  // 递归下载可能边扫描边发现文件，不能把当前文件字节当成整个目录进度。
  if (
    value.completedFiles !== undefined &&
    (value.totalFiles === undefined || value.totalFiles === 0)
  )
    return null;
  if (value.totalFiles !== undefined && value.totalFiles > 1)
    return value.completedFiles === undefined
      ? null
      : Math.min(
          100,
          Math.max(0, (value.completedFiles / value.totalFiles) * 100),
        );
  if (
    value.total &&
    /^\d+$/.test(value.total) &&
    /^\d+$/.test(value.transferred)
  ) {
    const total = BigInt(value.total);
    if (total > 0n) {
      const percentage = (BigInt(value.transferred) * 10000n) / total;
      return Math.min(
        100,
        Number(percentage > 10000n ? 10000n : percentage) / 100,
      );
    }
  }
  if (value.totalFiles && value.completedFiles !== undefined)
    return Math.min(
      100,
      Math.max(0, (value.completedFiles / value.totalFiles) * 100),
    );
  return null;
}
