#!/usr/bin/env python3
"""
从 update.md 提取指定版本更新日志，并生成格式化 GitHub Release 说明正文。
支持手动指定版本号、标题、下载直链及 SHA-256 校验清单。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")


HEADING_PATTERN = re.compile(
    r"^##\s+(?:\[?v?(?P<version>[0-9A-Za-z._-]+)\]?)(?:\s*-\s*(?P<date>\d{4}-\d{2}-\d{2}))?.*$"
)


def normalize_version(version: str) -> str:
    """去除版本号前缀 v/V，统一比较口径。"""
    return version.removeprefix("v").removeprefix("V").strip()


def extract_changelog(changelog_file: Path, version: str) -> str:
    """从更新日志文件中抽取目标版本的章节内容。"""
    target_version = normalize_version(version)
    if not changelog_file.is_file():
        return f"### 更新说明\n- 自动化构建发布版本 v{target_version}\n"

    lines = changelog_file.read_text(encoding="utf-8").splitlines()
    collecting = False
    collected: list[str] = []

    for line in lines:
        heading_match = HEADING_PATTERN.match(line.strip())
        if heading_match:
            current_version = normalize_version(heading_match.group("version"))
            if collecting:
                break
            collecting = current_version == target_version

        if collecting:
            collected.append(line)

    while collected and not collected[-1].strip():
        collected.pop()

    if not collected:
        # 如果未找到匹配版本，生成优雅兜底更新说明
        return (
            f"### 更新说明\n"
            f"- 自动化持续构建发布版本 `v{target_version}`\n"
            f"- 包含最新的功能修复与性能改进，详情请查看 Commit 记录。\n"
        )

    return "\n".join(collected).strip() + "\n"


def build_release_notes(
    title: str,
    version: str,
    changelog: str,
    download_url: str,
    sha256_content: str = "",
) -> str:
    """生成标准化 GitHub Release 说明正文。"""
    clean_ver = normalize_version(version)
    notes_parts: list[str] = [
        f"# {title}\n",
        f"Windows 桌面端原生高拟真英语测评辅助工具。\n",
        f"## 📦 产物下载\n",
        f"- **标准绿色包 (推荐)**: [`fepc-v{clean_ver}-windows-x64.zip`]({download_url})\n",
        f"- **单文件便携版 (加壳)**: [`fepc.exe`]({download_url})\n",
        f"- **官网直链**: [lastudio.cc/dl/fepc/latest](https://lastudio.cc/dl/fepc/latest)\n",
        f"\n## 📝 本次更新内容\n\n{changelog}\n",
    ]

    if sha256_content.strip():
        notes_parts.append(
            f"## 🔒 数字签名与哈希校验 (SHA-256)\n\n"
            f"```text\n{sha256_content.strip()}\n```\n\n"
        )

    notes_parts.append(
        "## ⚠️ 使用须知与法律免责声明\n"
        "- 本软件受《LA Studio 专有软件服务协议与商业软件许可 (EULA)》约束。\n"
        "- 本软件研发仅供计算机通信逆向研究与个人口语听力发音学习参考。\n"
        "- **严禁**将本软件用于任何正式考试、学业水平测试、日常作业考核舞弊等学术不端行为！违规后果由违规用户完全自行承担。\n"
        "- 云端模式登录时如遇官方客户端顶号，属于第三方服务正常单点登录机制，请避开考试等关键场景。\n"
    )

    return "\n".join(notes_parts)


def write_github_output(key: str, value: str, output_file: Path | None) -> None:
    """写入 GitHub Actions Step Output。"""
    if output_file is None:
        return

    with output_file.open("a", encoding="utf-8") as output:
        if "\n" in value:
            output.write(f"{key}<<EOF\n{value}\nEOF\n")
        else:
            output.write(f"{key}={value}\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="提取版本更新日志并生成 Release 正文")
    parser.add_argument("--changelog-file", default="update.md", help="更新日志文件路径")
    parser.add_argument("--version", required=True, help="目标发布版本号")
    parser.add_argument("--title", default="", help="Release 标题")
    parser.add_argument(
        "--download-url",
        default="https://github.com/laststudio/Fuck_ets100_pc/releases",
        help="产物下载地址前缀或仓库 Release 链接",
    )
    parser.add_argument("--sha256-file", default=None, help="包含 SHA-256 校验和的文件路径")
    parser.add_argument("--release-notes", default="release_notes.md", help="Release 说明正文输出文件")
    parser.add_argument("--changelog-output", default="current_changelog.md", help="当前版本日志输出文件")
    parser.add_argument("--github-output", default=None, help="GitHub Actions 输出环境变量文件")
    args = parser.parse_args()

    changelog_file = Path(args.changelog_file).resolve()
    release_notes_file = Path(args.release_notes).resolve()
    changelog_output_file = Path(args.changelog_output).resolve()
    output_file = Path(args.github_output).resolve() if args.github_output else None

    clean_ver = normalize_version(args.version)
    title = args.title.strip() if args.title.strip() else f"Fe PC v{clean_ver}"

    sha256_content = ""
    if args.sha256_file:
        sha_path = Path(args.sha256_file).resolve()
        if sha_path.is_file():
            sha256_content = sha_path.read_text(encoding="utf-8")

    try:
        changelog = extract_changelog(changelog_file, clean_ver)
        release_notes = build_release_notes(
            title=title,
            version=clean_ver,
            changelog=changelog,
            download_url=args.download_url,
            sha256_content=sha256_content,
        )
    except Exception as exc:
        print(f"[!] 提取更新日志遇到异常：{exc}", file=sys.stderr)
        return 1

    release_notes_file.parent.mkdir(parents=True, exist_ok=True)
    release_notes_file.write_text(release_notes, encoding="utf-8")

    changelog_output_file.parent.mkdir(parents=True, exist_ok=True)
    changelog_output_file.write_text(changelog, encoding="utf-8")

    write_github_output("release_title", title, output_file)
    write_github_output("release_version", clean_ver, output_file)

    print(f"[+] 成功生成发布说明：{release_notes_file}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
