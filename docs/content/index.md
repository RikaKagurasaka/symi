---
seo:
  title: Symi 编辑器文档
  description: Symi 编辑器是一款基于文本的微分音乐编辑器，支持多种音高和节奏表记，适合实验性音乐创作。
  ogTitle: Symi 编辑器文档
  ogDescription: Symi 编辑器是一款基于文本的微分音乐编辑器，支持多种音高和节奏表记，适合实验性音乐创作。
---

::u-page-hero{class="dark:bg-gradient-to-b from-neutral-900 to-neutral-950"}
---
orientation: horizontal
---
#top
:hero-background

#title
基于文本的 <br/>[微分音乐编辑器]{.text-primary}

#description
Symi 是一种文本标记语言，用于描述音高和节奏，受 [simai语](https://w.atwiki.jp/simai/pages/1002.html){.text-primary target=_blank} 启发创作。其提供了多种表达方式，支持微分音和复杂节奏结构。

Symi编辑器支持实时回放、钢琴窗预览和导出MIDI，适合实验性音乐创作。

#links
  :::u-button
  ---
  to: /getting-started
  size: xl
  trailing-icon: i-lucide-arrow-right
  ---
  了解更多
  :::

  :::u-button
  ---
  to: /getting-started/installation
  color: neutral
  variant: outline
  size: xl
  icon: i-lucide-download
  ---
  下载安装
  :::


#default
  ![Example1](/images/example1.png)
::

::u-page-section{class="dark:bg-neutral-950"}
#title
主要功能

#links
  :::u-button
  ---
  color: neutral
  size: lg
  target: _blank
  to: https://ui.nuxt.com/docs/getting-started/installation/nuxt
  trailingIcon: i-lucide-arrow-right
  variant: subtle
  ---
  速速阅读文档
  :::

#features
  :::u-page-feature
  ---
  icon: i-lucide-keyboard-music
  ---
  #title
  多种音高表记

  #description
  支持多种音高表示方法，包括频率、倍音、平均律、音分和音名等，甚至链接多种表示。
  :::

  :::u-page-feature
  ---
  icon: i-lucide-drum
  ---
  #title
  多种节奏表记

  #description
  允许设置反常拍号、支持任意有理数比例的量化，还有语法糖来简化复杂节奏的表达。
  :::

  :::u-page-feature
  ---
  icon: i-lucide-guitar
  ---
  #title
  和弦/多声部

  #description
  支持同时演奏多个音符，即使音高和节奏都不同。
  :::

  :::u-page-feature
  ---
  icon: i-lucide-piano
  ---
  #title
  多彩钢琴窗

  #description
  即使是微分音，也可以在钢琴窗中可视化，并且以颜色标记音高，更直观地理解和声结构。
  :::

  :::u-page-feature
  ---
  icon: i-lucide-square-arrow-out-up-right
  ---
  #title
  导出 MIDI

  #description
  没错！可以通过多轨和弯音参数将微分音乐导出为 MIDI 文件，方便在其他 DAW 中使用。
  :::

  :::u-page-feature
  ---
  icon: i-lucide-ellipsis
  ---
  #title
  更多

  #description
  该项目仍在锐意开发中，提出建议可能被采纳！
  :::
::

::u-page-section{class="dark:bg-gradient-to-b from-neutral-950 to-neutral-900"}
  :::u-page-c-t-a
  ---
  links:
    - label: 加入Discord
      to: 'https://discord.gg/pyZYtqXjeB'
      target: _blank
      variant: subtle
      icon: i-simple-icons-discord
    - label: GitHub Issues
      to: 'https://github.com/RikaKagurasaka/symi/issues'
      target: _blank
      variant: subtle
      icon: i-simple-icons-github
  title: 还有想说的……
  description: 可以加入Discord，或在 GitHub 上提出 issue。
  class: dark:bg-neutral-950
  ---

  :stars-bg
  :::
::
