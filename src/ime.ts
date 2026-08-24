/**
 * 输入法组合期间的回车是「上屏」操作（如拼音打英文时确认字母原文），
 * 不应触发提交；keyCode 229 兼容未正确设置 isComposing 的部分输入法
 */
export function onEnterSubmit(e: KeyboardEvent, submit: () => void): void {
  if (e.isComposing || e.keyCode === 229) return
  submit()
}
