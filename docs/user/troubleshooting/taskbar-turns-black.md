---
title: The Windows taskbar turns black when FDE2E Photocraft is maximized
description: FDE2E Photocraft inherits the Windows taskbar fix introduced in upstream PhotoCraft 0.8.2.
order: 10
verified: 0.8.2
---

Update to a current FDE2E Photocraft build. Upstream PhotoCraft 0.7.0 on Windows 11 could turn the taskbar black or hide it while the window was maximized. This fork inherits the fix introduced in upstream PhotoCraft 0.8.2.

## Symptoms

- You maximize the FDE2E Photocraft window, or switch it to full screen, and the Windows taskbar turns completely black or is hidden.
- After you resize the window and maximize it again, a black bar the height of the taskbar stays at the bottom of the screen.

## Fix

1. Check your version with **Help › System Info…**, which starts with "FDE2E Photocraft" and the version.
2. If it is older than 0.8.2, download the [latest release](https://github.com/jixoai/photocraft/releases/latest). See [Install FDE2E Photocraft](../getting-started/install.md).
3. Start FDE2E Photocraft again and maximize the window.

A user who saw the black taskbar on 0.7.0 confirmed that it is gone in 0.8.2 ([issue 3864](https://github.com/storytold/photocraft/issues/3864)).

## Why this happens

FDE2E Photocraft draws its own title bar. When such a window was maximized, Windows sized it to cover the whole screen, taskbar included, took it for a full-screen app and dropped the taskbar's background. The fix keeps a maximized FDE2E Photocraft window clear of the taskbar ([issue 3144](https://github.com/storytold/photocraft/issues/3144), [pull request 3308](https://github.com/storytold/photocraft/pull/3308)).

## Still stuck?

If you still see it on 0.8.2, open an issue in [this fork](https://github.com/jixoai/photocraft/issues) with your Windows version and FDE2E Photocraft version. As a test, you can switch on **System title bar** under **Edit › Preferences › Interface…**, click **Apply** and restart FDE2E Photocraft; Windows then draws the title bar. See [Get help with a problem](get-help.md) for what to include.
