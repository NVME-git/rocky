import 'dart:ui_web' as ui_web;
// ignore: avoid_web_libraries_in_flutter, deprecated_member_use
import 'dart:html' as html;

import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:url_launcher/url_launcher.dart';
import 'content.dart';

void main() {
  runApp(const RockyDocsApp());
}

// ---------------------------------------------------------------------------
// Global theme state
// ---------------------------------------------------------------------------
final ValueNotifier<bool> _isDark = ValueNotifier(true);

// ---------------------------------------------------------------------------
// Color palette — dynamic based on theme
// ---------------------------------------------------------------------------
class AppColors {
  static bool get _d => _isDark.value;

  static Color get background => _d ? const Color(0xFF0A0E1A) : const Color(0xFFF8FAFC);
  static Color get surface    => _d ? const Color(0xFF111827) : const Color(0xFFFFFFFF);
  static Color get sidebarBg  => _d ? const Color(0xFF070B14) : const Color(0xFFF1F5F9);
  static Color get sidebarHover => _d ? const Color(0xFF0F1629) : const Color(0xFFE2E8F0);
  static Color get primary    => _d ? const Color(0xFFF59E0B) : const Color(0xFFD97706);
  static Color get primaryDim => _d ? const Color(0xFFD97706) : const Color(0xFFB45309);
  static Color get secondary  => _d ? const Color(0xFF06B6D4) : const Color(0xFF0891B2);
  static Color get textPrimary   => _d ? const Color(0xFFF1F5F9) : const Color(0xFF0F172A);
  static Color get textSecondary => _d ? const Color(0xFFCBD5E1) : const Color(0xFF334155);
  static Color get textMuted     => _d ? const Color(0xFF64748B) : const Color(0xFF94A3B8);
  static Color get codeBg        => _d ? const Color(0xFF1E293B) : const Color(0xFFF1F5F9);
  static Color get codeBorder    => _d ? const Color(0xFF334155) : const Color(0xFFCBD5E1);
  static Color get divider       => _d ? const Color(0xFF1E293B) : const Color(0xFFE2E8F0);
  static Color get cardBg        => _d ? const Color(0xFF0F172A) : const Color(0xFFF8FAFC);
}

// Terminal block colours — always dark regardless of theme.
// Semantic colors match the Rocky CLI truecolor values exactly.
class TermColors {
  static const bg         = Color(0xFF0d1117);
  static const headerBg   = Color(0xFF161b22);
  static const border     = Color(0xFF30363d);
  static const prompt     = Color(0xFF1D9E75);  // #1D9E75 — app success/known green
  static const cmdText    = Color(0xFFf0f6fc);  // bright command text
  static const commentTxt = Color(0xFF64748B);  // muted slate
  static const outputTxt  = Color(0xFFCBD5E1);  // normal output
  static const cursor     = Color(0xFF1D9E75);
  static const dot1       = Color(0xFFFF5F57);
  static const dot2       = Color(0xFFFFBD2E);
  static const dot3       = Color(0xFF28C840);
  static const titleTxt   = Color(0xFF8b949e);

  // Semantic: match the Rocky CLI truecolors
  static const rockyBrand   = Color(0xFFF59E0B);  // #F59E0B — Rocky brand amber (banner, personality)
  static const rockyFeedback= Color(0xFF06B6D4);  // #06B6D4 — Rocky's voice/feedback (cyan)
  static const successGreen = Color(0xFF1D9E75);  // #1D9E75 — ✓ success / known
  static const fadingAmber  = Color(0xFFEF9F27);  // #EF9F27 — ~ fading / warning
  static const gapRed       = Color(0xFFE24B4A);  // #E24B4A — ✗ gap / error
  static const userAnswer   = Color(0xFFF1F5F9);  // bright white — user's own words
  static const hintMuted    = Color(0xFF475569);  // dimmed hint text
}

// ---------------------------------------------------------------------------
// App root
// ---------------------------------------------------------------------------
class RockyDocsApp extends StatelessWidget {
  const RockyDocsApp({super.key});

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (context, dark, _) {
        return MaterialApp(
          title: 'Rocky',
          debugShowCheckedModeBanner: false,
          theme: (dark ? ThemeData.dark(useMaterial3: true) : ThemeData.light(useMaterial3: true)).copyWith(
            scaffoldBackgroundColor: AppColors.background,
            colorScheme: dark
                ? ColorScheme.dark(
                    primary: AppColors.primary,
                    secondary: AppColors.secondary,
                    surface: AppColors.surface,
                  )
                : ColorScheme.light(
                    primary: AppColors.primary,
                    secondary: AppColors.secondary,
                    surface: AppColors.surface,
                  ),
            dividerColor: AppColors.divider,
            textTheme: (dark ? ThemeData.dark() : ThemeData.light()).textTheme.apply(
              bodyColor: AppColors.textPrimary,
              displayColor: AppColors.textPrimary,
              fontFamily: 'Roboto',
            ),
          ),
          home: const DocsShell(),
        );
      },
    );
  }
}

// ---------------------------------------------------------------------------
// Section model
// ---------------------------------------------------------------------------
class DocSection {
  final String title;
  final IconData icon;
  final String markdown;
  const DocSection(this.title, this.icon, this.markdown);
}

final List<DocSection> kSections = [
  const DocSection('What is Rocky?', Icons.auto_awesome, kIntroduction),
  const DocSection('Installation', Icons.download_rounded, kInstallation),
  const DocSection('Quick Start', Icons.rocket_launch, kQuickstart),
  const DocSection('Commands', Icons.terminal, kCommands),
  const DocSection('Configuration', Icons.settings, kConfiguration),
  const DocSection('How It Works', Icons.account_tree, kHowItWorks),
  const DocSection('Sync & Backup', Icons.sync, kSync),
  const DocSection('Obsidian', Icons.hub, kObsidian),
  const DocSection('Demo', Icons.play_circle_outline_rounded, kWalkthrough),
  const DocSection('Voice', Icons.mic_rounded, kVoice),
  const DocSection('References', Icons.menu_book_outlined, kReferences),
];

// ---------------------------------------------------------------------------
// App Shell — sidebar + content
// ---------------------------------------------------------------------------
class DocsShell extends StatefulWidget {
  const DocsShell({super.key});
  @override
  State<DocsShell> createState() => _DocsShellState();
}

class _DocsShellState extends State<DocsShell> {
  int _selected = 0;
  final _scrollController = ScrollController();

  static const double _sidebarWidth = 280;
  static const double _breakpoint = 768;

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  void _selectSection(int index) {
    setState(() => _selected = index);
    _scrollController.jumpTo(0);
    if (_isNarrow(context)) Navigator.of(context).pop();
  }

  bool _isNarrow(BuildContext context) =>
      MediaQuery.of(context).size.width < _breakpoint;

  @override
  Widget build(BuildContext context) {
    final narrow = _isNarrow(context);
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (context, _, __) => Scaffold(
        backgroundColor: AppColors.background,
        appBar: narrow
            ? AppBar(
                backgroundColor: AppColors.sidebarBg,
                title: const _LogoRow(compact: true),
                elevation: 0,
                actions: [
                  _ThemeToggleButton(),
                  const SizedBox(width: 8),
                ],
              )
            : null,
        drawer: narrow ? Drawer(child: _buildSidebar()) : null,
        body: Row(
          children: [
            if (!narrow) _buildSidebar(),
            Expanded(child: _buildContent()),
          ],
        ),
      ),
    );
  }

  Widget _buildSidebar() {
    return Container(
      width: _sidebarWidth,
      decoration: BoxDecoration(
        color: AppColors.sidebarBg,
        border: Border(right: BorderSide(color: AppColors.divider, width: 1)),
      ),
      child: Column(
        children: [
          const SizedBox(height: 24),
          const _LogoRow(),
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Text(
              'You Observe. Question?',
              style: TextStyle(
                color: AppColors.textMuted,
                fontSize: 12,
                letterSpacing: 0.5,
              ),
            ),
          ),
          const SizedBox(height: 24),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Divider(color: AppColors.divider, height: 1),
          ),
          const SizedBox(height: 12),
          Expanded(
            child: ListView.builder(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
              itemCount: kSections.length,
              itemBuilder: (context, i) {
                final section = kSections[i];
                final active = i == _selected;
                return _SidebarItem(
                  icon: section.icon,
                  label: section.title,
                  active: active,
                  onTap: () => _selectSection(i),
                );
              },
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Divider(color: AppColors.divider, height: 1),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(Icons.code, size: 14, color: AppColors.textMuted),
                const SizedBox(width: 8),
                GestureDetector(
                  onTap: () => _openUrl('https://github.com/NVME-git/rocky'),
                  child: Text(
                    'GitHub',
                    style: TextStyle(color: AppColors.textMuted, fontSize: 12),
                  ),
                ),
                const Spacer(),
                _ThemeToggleButton(),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildContent() {
    return Container(
      color: AppColors.background,
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 860),
          child: Scrollbar(
            controller: _scrollController,
            thumbVisibility: true,
            child: SingleChildScrollView(
              controller: _scrollController,
              padding: const EdgeInsets.symmetric(horizontal: 48, vertical: 40),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (_selected == 0)
                    _HeroBlock(
                      onInstall: () => _selectSection(1),
                      onLearnMore: () => _scrollController.animateTo(
                        560,
                        duration: const Duration(milliseconds: 600),
                        curve: Curves.easeInOutCubic,
                      ),
                    ),
                  SectionContent(
                    key: ValueKey(_selected),
                    markdown: kSections[_selected].markdown,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Dark / light mode toggle button
// ---------------------------------------------------------------------------
class _ThemeToggleButton extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (_, dark, __) => Tooltip(
        message: dark ? 'Switch to light mode' : 'Switch to dark mode',
        child: InkWell(
          onTap: () => _isDark.value = !_isDark.value,
          borderRadius: BorderRadius.circular(20),
          child: Padding(
            padding: const EdgeInsets.all(6),
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 300),
              child: Icon(
                dark ? Icons.light_mode_rounded : Icons.dark_mode_rounded,
                key: ValueKey(dark),
                size: 18,
                color: AppColors.textMuted,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Logo row
// ---------------------------------------------------------------------------
class _LogoRow extends StatelessWidget {
  final bool compact;
  const _LogoRow({this.compact = false});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: compact ? 0 : 24),
      child: Row(
        mainAxisSize: compact ? MainAxisSize.min : MainAxisSize.max,
        children: [
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(
              gradient: LinearGradient(
                colors: [AppColors.primary, AppColors.primaryDim],
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
              ),
              borderRadius: BorderRadius.circular(8),
            ),
            child: const Center(
              child: Text('🪨',
                  style: TextStyle(fontSize: 16, color: Colors.black)),
            ),
          ),
          const SizedBox(width: 12),
          Text(
            'Rocky',
            style: TextStyle(
              color: AppColors.textPrimary,
              fontSize: 20,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.5,
            ),
          ),
          const SizedBox(width: 6),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: AppColors.primary.withValues(alpha: 0.15),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Text(
              'docs',
              style: TextStyle(
                color: AppColors.primary,
                fontSize: 11,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Sidebar item
// ---------------------------------------------------------------------------
class _SidebarItem extends StatefulWidget {
  final IconData icon;
  final String label;
  final bool active;
  final VoidCallback onTap;
  const _SidebarItem({
    required this.icon,
    required this.label,
    required this.active,
    required this.onTap,
  });
  @override
  State<_SidebarItem> createState() => _SidebarItemState();
}

class _SidebarItemState extends State<_SidebarItem> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final bg = widget.active
        ? AppColors.primary.withValues(alpha: 0.12)
        : _hovered
            ? AppColors.sidebarHover
            : Colors.transparent;
    final fg = widget.active ? AppColors.primary : AppColors.textSecondary;

    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          margin: const EdgeInsets.symmetric(vertical: 2),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(8),
            border: widget.active
                ? Border.all(color: AppColors.primary.withValues(alpha: 0.25))
                : null,
          ),
          child: Row(
            children: [
              Icon(widget.icon, size: 18, color: fg),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  widget.label,
                  style: TextStyle(
                    color: fg,
                    fontSize: 14,
                    fontWeight:
                        widget.active ? FontWeight.w600 : FontWeight.w400,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Markdown part model — text vs fenced code block
// ---------------------------------------------------------------------------
sealed class _MdPart {}

class _TextPart extends _MdPart {
  final String text;
  _TextPart(this.text);
}

class _CodePart extends _MdPart {
  final String code;
  final String language;
  _CodePart(this.code, this.language);
}

class _DetailsPart extends _MdPart {
  final String title;
  final String body;
  _DetailsPart(this.title, this.body);
}

List<_MdPart> _parseParts(String markdown) {
  // First pass: extract :::details TITLE\n...\n::: blocks.
  // The body may itself contain code fences, so we recurse into _parseCodeAndText
  // for everything outside the details blocks and store the body as raw markdown.
  final parts = <_MdPart>[];
  final detailsRe = RegExp(r':::details\s+([^\n]+)\n([\s\S]*?)\n:::', multiLine: true);
  int lastEnd = 0;
  for (final m in detailsRe.allMatches(markdown)) {
    if (m.start > lastEnd) {
      parts.addAll(_parseCodeAndText(markdown.substring(lastEnd, m.start)));
    }
    parts.add(_DetailsPart((m.group(1) ?? '').trim(), m.group(2) ?? ''));
    lastEnd = m.end;
  }
  if (lastEnd < markdown.length) {
    parts.addAll(_parseCodeAndText(markdown.substring(lastEnd)));
  }
  return parts;
}

List<_MdPart> _parseCodeAndText(String markdown) {
  final parts = <_MdPart>[];
  // Match fenced code blocks: ```lang\n...\n```
  final re = RegExp(r'```(\w*)\n([\s\S]*?)```', multiLine: true);
  int lastEnd = 0;
  for (final m in re.allMatches(markdown)) {
    if (m.start > lastEnd) {
      parts.add(_TextPart(markdown.substring(lastEnd, m.start)));
    }
    final code = m.group(2) ?? '';
    // Trim trailing newline that the regex captures
    parts.add(_CodePart(code.endsWith('\n') ? code.substring(0, code.length - 1) : code, m.group(1) ?? ''));
    lastEnd = m.end;
  }
  if (lastEnd < markdown.length) {
    parts.add(_TextPart(markdown.substring(lastEnd)));
  }
  return parts;
}

// ---------------------------------------------------------------------------
// Section content — mixes StyledMarkdown and TerminalBlock
// ---------------------------------------------------------------------------
class SectionContent extends StatelessWidget {
  final String markdown;
  const SectionContent({super.key, required this.markdown});

  @override
  Widget build(BuildContext context) {
    final parts = _parseParts(markdown);
    final widgets = <Widget>[];
    int i = 0;
    while (i < parts.length) {
      final part = parts[i];
      if (part is _TextPart) {
        widgets.add(StyledMarkdown(data: part.text));
        i++;
      } else if (part is _DetailsPart) {
        widgets.add(Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: _DetailsPanel(title: part.title, body: part.body),
        ));
        i++;
      } else if (part is _CodePart && part.language == 'youtube') {
        widgets.add(Padding(
          padding: const EdgeInsets.symmetric(vertical: 12),
          child: _YouTubeEmbed(videoId: part.code.trim()),
        ));
        i++;
      } else if (part is _CodePart) {
        // Combine a bash input block with the immediately following plain
        // output block into a single terminal: input typed, output instant.
        String? outputCode;
        if (part.language == 'bash' &&
            i + 1 < parts.length &&
            parts[i + 1] is _CodePart &&
            (parts[i + 1] as _CodePart).language.isEmpty) {
          outputCode = (parts[i + 1] as _CodePart).code;
          i++; // consume the output block
        }
        widgets.add(Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: TerminalBlock(
            code: part.code,
            language: part.language,
            outputCode: outputCode,
          ),
        ));
        i++;
      } else {
        i++;
      }
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
  }
}

// ---------------------------------------------------------------------------
// Terminal block — animated, blinking cursor, Mac chrome
// ---------------------------------------------------------------------------
class TerminalBlock extends StatelessWidget {
  final String code;
  final String language;
  /// Output rendered after the input section. Static — no reveal animation.
  final String? outputCode;
  const TerminalBlock({super.key, required this.code, required this.language, this.outputCode});

  /// Split the source at the last shell-prompt line. Everything up to and
  /// including that line is the input section; everything after is output.
  /// If no prompt is found (or the prompt is on the last line), all of `code`
  /// is treated as input and `outputCode` (if given) is the output.
  ({String input, String output}) _split() {
    final promptRe = RegExp(r'^[\w~/.]*\s*\$\s');
    final lines = code.split('\n');
    int lastPrompt = -1;
    for (int i = 0; i < lines.length; i++) {
      if (promptRe.hasMatch(lines[i])) lastPrompt = i;
    }
    if (lastPrompt < 0 || lastPrompt == lines.length - 1) {
      return (input: code, output: outputCode ?? '');
    }
    final input = lines.sublist(0, lastPrompt + 1).join('\n');
    final rest = lines.sublist(lastPrompt + 1).join('\n');
    final output = outputCode != null ? '$rest\n$outputCode' : rest;
    return (input: input, output: output);
  }

  String get _title => language.isEmpty ? 'output' : language;

  @override
  Widget build(BuildContext context) {
    final sections = _split();
    return Container(
      decoration: BoxDecoration(
        color: TermColors.bg,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: TermColors.border, width: 1),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.4),
            blurRadius: 16,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildTitleBar(),
          _buildBody(sections.input, sections.output),
        ],
      ),
    );
  }

  Widget _buildTitleBar() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: const BoxDecoration(
        color: TermColors.headerBg,
        borderRadius: BorderRadius.only(
          topLeft: Radius.circular(10),
          topRight: Radius.circular(10),
        ),
        border: Border(bottom: BorderSide(color: TermColors.border, width: 1)),
      ),
      child: Row(
        children: [
          _dot(TermColors.dot1),
          const SizedBox(width: 6),
          _dot(TermColors.dot2),
          const SizedBox(width: 6),
          _dot(TermColors.dot3),
          const SizedBox(width: 14),
          Text(
            _title,
            style: const TextStyle(
              color: TermColors.titleTxt,
              fontFamily: 'monospace',
              fontSize: 12,
              letterSpacing: 0.5,
            ),
          ),
        ],
      ),
    );
  }

  Widget _dot(Color color) => Container(
        width: 12,
        height: 12,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      );

  Widget _buildBody(String inputSection, String outputSection) {
    final inputLines = inputSection.split('\n');
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: const BoxDecoration(
        color: TermColors.bg,
        borderRadius: BorderRadius.only(
          bottomLeft: Radius.circular(10),
          bottomRight: Radius.circular(10),
        ),
      ),
      child: SelectionArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final line in inputLines) _buildLine(line),
            if (outputSection.isNotEmpty) ...[
              const SizedBox(height: 2),
              ..._buildOutputLines(outputSection),
            ],
            // Static cursor at the end — no blink, no reveal animation.
            const Text(
              '█',
              style: TextStyle(
                color: TermColors.cursor,
                fontFamily: 'monospace',
                fontSize: 14,
                height: 1.0,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildLine(String line) {
    return Text.rich(
      _styleLine(line),
      style: const TextStyle(
        fontFamily: 'monospace',
        fontSize: 13.5,
        height: 1.55,
      ),
    );
  }

  /// Style a single terminal line (used for the typing-animation input section).
  TextSpan _styleLine(String line) {
    final trimmed = line.trimLeft();
    final promptMatch = RegExp(r'^([\w~/.]*\s*\$\s+)(.*)').firstMatch(line);
    if (promptMatch != null) {
      return TextSpan(children: [
        TextSpan(text: promptMatch.group(1),
            style: const TextStyle(color: TermColors.prompt, fontWeight: FontWeight.w600)),
        TextSpan(text: promptMatch.group(2),
            style: const TextStyle(color: TermColors.cmdText)),
      ]);
    }
    if (trimmed.startsWith('#')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.commentTxt));
    }
    return TextSpan(text: line,
        style: const TextStyle(color: TermColors.outputTxt));
  }

  /// Stateful output renderer — tracks question/answer blocks across lines so
  /// every continuation line of a question is cyan and every line of an answer
  /// is white, not just the first.
  List<Widget> _buildOutputLines(String output) {
    // _LineMode: 0 = normal, 1 = question, 2 = answer
    int mode = 0;
    final widgets = <Widget>[];

    for (final line in output.split('\n')) {
      final trimmed = line.trimLeft();
      TextSpan span;

      // --- Lines that unconditionally reset mode ---
      final promptMatch = RegExp(r'^([\w~/.]*\s*\$\s+)(.*)').firstMatch(line);
      if (promptMatch != null) {
        mode = 0;
        span = TextSpan(children: [
          TextSpan(text: promptMatch.group(1),
              style: const TextStyle(color: TermColors.prompt, fontWeight: FontWeight.w600)),
          TextSpan(text: promptMatch.group(2),
              style: const TextStyle(color: TermColors.cmdText)),
        ]);
      } else if (trimmed.startsWith('Rocky:')) {
        mode = 0;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.rockyBrand, fontWeight: FontWeight.w500));
      } else if (trimmed.startsWith('✓')) {
        mode = 0;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.successGreen));
      } else if (trimmed.startsWith('✗')) {
        mode = 0;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.gapRed));
      } else if (trimmed.startsWith('~')) {
        mode = 0;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.fadingAmber));

      // --- Mode starters ---
      } else if (RegExp(r'^Q\d+\.').hasMatch(trimmed)) {
        mode = 1;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.rockyFeedback));
      } else if (trimmed.startsWith('>')) {
        mode = 2;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.userAnswer, fontStyle: FontStyle.italic));

      // --- Blank lines: reset mode, render plain ---
      } else if (trimmed.isEmpty) {
        mode = 0;
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.outputTxt));

      // --- Hint / muted lines (do not change mode) ---
      } else if (trimmed.startsWith('[')) {
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.hintMuted));
      } else if (trimmed.startsWith('Evaluating') ||
                 trimmed.startsWith('Fetching') ||
                 trimmed.startsWith('Analyzing') ||
                 trimmed.startsWith('Saved to PKG')) {
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.hintMuted));
      } else if (trimmed.startsWith('#')) {
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.commentTxt));

      // --- Continuation: inherit current mode ---
      } else if (mode == 1) {
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.rockyFeedback));
      } else if (mode == 2) {
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.userAnswer, fontStyle: FontStyle.italic));
      } else {
        span = TextSpan(text: line,
            style: const TextStyle(color: TermColors.outputTxt));
      }

      widgets.add(Text.rich(
        span,
        style: const TextStyle(fontFamily: 'monospace', fontSize: 13.5, height: 1.55),
      ));
    }
    return widgets;
  }
}

// ---------------------------------------------------------------------------
// Styled Markdown renderer (non-code-block text only)
// ---------------------------------------------------------------------------
class StyledMarkdown extends StatelessWidget {
  final String data;
  const StyledMarkdown({super.key, required this.data});

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (context, _, __) => MarkdownBody(
        data: data,
        selectable: true,
        onTapLink: (text, href, title) {
          if (href != null) _openUrl(href);
        },
        styleSheet: MarkdownStyleSheet(
          h1: TextStyle(
            color: AppColors.textPrimary,
            fontSize: 32,
            fontWeight: FontWeight.w800,
            height: 1.3,
            letterSpacing: -0.5,
          ),
          h1Padding: const EdgeInsets.only(bottom: 16, top: 8),
          h2: TextStyle(
            color: AppColors.textPrimary,
            fontSize: 24,
            fontWeight: FontWeight.w700,
            height: 1.4,
          ),
          h2Padding: const EdgeInsets.only(bottom: 12, top: 32),
          h3: TextStyle(
            color: AppColors.textPrimary,
            fontSize: 18,
            fontWeight: FontWeight.w600,
            height: 1.4,
          ),
          h3Padding: const EdgeInsets.only(bottom: 8, top: 24),
          p: TextStyle(
            color: AppColors.textSecondary,
            fontSize: 15,
            height: 1.7,
          ),
          pPadding: const EdgeInsets.only(bottom: 12),
          strong: TextStyle(
            color: AppColors.textPrimary,
            fontWeight: FontWeight.w700,
          ),
          em: TextStyle(
            color: AppColors.textSecondary,
            fontStyle: FontStyle.italic,
          ),
          a: TextStyle(
            color: AppColors.secondary,
            decoration: TextDecoration.underline,
            decorationColor: AppColors.secondary,
          ),
          code: TextStyle(
            fontFamily: 'monospace',
            fontSize: 13.5,
            color: AppColors.primary,
            backgroundColor: AppColors.codeBg,
          ),
          codeblockDecoration: BoxDecoration(
            color: AppColors.codeBg,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: AppColors.codeBorder, width: 1),
          ),
          codeblockPadding: const EdgeInsets.all(16),
          listBullet: TextStyle(color: AppColors.primary, fontSize: 14),
          listBulletPadding: const EdgeInsets.only(right: 8),
          listIndent: 24,
          blockquote: TextStyle(
            color: AppColors.textMuted,
            fontSize: 15,
            height: 1.7,
            fontStyle: FontStyle.italic,
          ),
          blockquoteDecoration: BoxDecoration(
            border: Border(
              left: BorderSide(color: AppColors.primary, width: 3),
            ),
            color: AppColors.primary.withValues(alpha: 0.06),
            borderRadius: const BorderRadius.only(
              topRight: Radius.circular(4),
              bottomRight: Radius.circular(4),
            ),
          ),
          blockquotePadding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
          horizontalRuleDecoration: BoxDecoration(
            border: Border(
              top: BorderSide(color: AppColors.divider, width: 1),
            ),
          ),
          tableHead: TextStyle(
            color: AppColors.textPrimary,
            fontWeight: FontWeight.w600,
            fontSize: 14,
          ),
          tableBody: TextStyle(
            color: AppColors.textSecondary,
            fontSize: 14,
            height: 1.5,
          ),
          tableBorder: TableBorder.all(
            color: AppColors.codeBorder,
            width: 1,
            borderRadius: BorderRadius.circular(4),
          ),
          tableHeadAlign: TextAlign.left,
          tableCellsPadding:
              const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// YouTube embed — for ```youtube VIDEO_ID``` fenced blocks.
// Renders a 16:9 iframe via HtmlElementView on Flutter web.
// Falls back to a "coming soon" card when the ID is a placeholder.
// ---------------------------------------------------------------------------
class _YouTubeEmbed extends StatelessWidget {
  final String videoId;
  const _YouTubeEmbed({required this.videoId});

  static final Set<String> _registered = {};

  bool get _isPlaceholder {
    final v = videoId.trim().toUpperCase();
    return v.isEmpty ||
        v == 'PLACEHOLDER' ||
        v == 'TODO' ||
        v.startsWith('TODO_') ||
        v.startsWith('PLACEHOLDER_');
  }

  void _registerOnce() {
    if (_registered.contains(videoId)) return;
    _registered.add(videoId);
    ui_web.platformViewRegistry.registerViewFactory('youtube-$videoId', (int _) {
      final iframe = html.IFrameElement()
        ..src = 'https://www.youtube-nocookie.com/embed/$videoId'
        ..style.border = 'none'
        ..style.width = '100%'
        ..style.height = '100%'
        ..allow = 'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture';
      iframe.setAttribute('allowfullscreen', 'true');
      return iframe;
    });
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (context, _, __) {
        if (_isPlaceholder) {
          return AspectRatio(
            aspectRatio: 16 / 9,
            child: Container(
              decoration: BoxDecoration(
                color: AppColors.cardBg,
                border: Border.all(color: AppColors.divider, width: 1),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.play_circle_outline_rounded,
                      size: 64,
                      color: AppColors.textMuted,
                    ),
                    const SizedBox(height: 14),
                    Text(
                      'Demo video coming soon',
                      style: TextStyle(
                        color: AppColors.textSecondary,
                        fontWeight: FontWeight.w600,
                        fontSize: 16,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'A walkthrough is in production',
                      style: TextStyle(
                        color: AppColors.textMuted,
                        fontSize: 12,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          );
        }

        _registerOnce();
        return AspectRatio(
          aspectRatio: 16 / 9,
          child: Container(
            decoration: BoxDecoration(
              color: Colors.black,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: AppColors.divider, width: 1),
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(10),
              child: HtmlElementView(viewType: 'youtube-$videoId'),
            ),
          ),
        );
      },
    );
  }
}

// ---------------------------------------------------------------------------
// Collapsible details panel — for ":::details TITLE\n...\n:::" blocks
// ---------------------------------------------------------------------------
class _DetailsPanel extends StatefulWidget {
  final String title;
  final String body;
  const _DetailsPanel({required this.title, required this.body});
  @override
  State<_DetailsPanel> createState() => _DetailsPanelState();
}

class _DetailsPanelState extends State<_DetailsPanel> {
  bool _open = false;
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (context, _, __) => Container(
        decoration: BoxDecoration(
          color: AppColors.cardBg,
          border: Border.all(color: AppColors.divider, width: 1),
          borderRadius: BorderRadius.circular(10),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            MouseRegion(
              cursor: SystemMouseCursors.click,
              onEnter: (_) => setState(() => _hovered = true),
              onExit: (_) => setState(() => _hovered = false),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => setState(() => _open = !_open),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                  color: _hovered ? AppColors.sidebarHover : Colors.transparent,
                  child: Row(
                    children: [
                      AnimatedRotation(
                        turns: _open ? 0.25 : 0.0,
                        duration: const Duration(milliseconds: 180),
                        child: Icon(
                          Icons.chevron_right_rounded,
                          size: 20,
                          color: AppColors.textMuted,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          widget.title,
                          style: TextStyle(
                            color: AppColors.textPrimary,
                            fontSize: 15,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      Text(
                        _open ? 'Hide' : 'Show',
                        style: TextStyle(
                          color: AppColors.textMuted,
                          fontSize: 12,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            if (_open)
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
                child: SectionContent(markdown: widget.body),
              ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Hero block — landing page above the "What is Rocky?" content
// ---------------------------------------------------------------------------
class _HeroBlock extends StatelessWidget {
  final VoidCallback onInstall;
  final VoidCallback onLearnMore;
  const _HeroBlock({required this.onInstall, required this.onLearnMore});

  @override
  Widget build(BuildContext context) {
    final isNarrow = MediaQuery.of(context).size.width < 900;
    final left = _HeroText(isNarrow: isNarrow, onInstall: onInstall, onLearnMore: onLearnMore);
    const right = _RockyIqDial(target: 74);

    final body = isNarrow
        ? Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              left,
              const SizedBox(height: 32),
              const Center(child: right),
            ],
          )
        : Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Expanded(flex: 5, child: left),
              const SizedBox(width: 32),
              right,
            ],
          );

    return Container(
      padding: const EdgeInsets.only(bottom: 40),
      margin: const EdgeInsets.only(bottom: 32),
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: AppColors.divider, width: 1)),
      ),
      child: body,
    );
  }
}

class _HeroText extends StatelessWidget {
  final bool isNarrow;
  final VoidCallback onInstall;
  final VoidCallback onLearnMore;
  const _HeroText({required this.isNarrow, required this.onInstall, required this.onLearnMore});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
          decoration: BoxDecoration(
            color: AppColors.primary.withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(99),
            border: Border.all(color: AppColors.primary.withValues(alpha: 0.3), width: 1),
          ),
          child: Text(
            '♫  ROCKY IQ',
            style: TextStyle(
              color: AppColors.primary,
              fontSize: 11,
              fontWeight: FontWeight.w700,
              letterSpacing: 1.2,
            ),
          ),
        ),
        const SizedBox(height: 20),
        Text(
          "What's your Rocky IQ?",
          style: TextStyle(
            color: AppColors.textPrimary,
            fontSize: isNarrow ? 36 : 44,
            fontWeight: FontWeight.w800,
            height: 1.1,
            letterSpacing: -1,
          ),
        ),
        const SizedBox(height: 16),
        Text(
          'A live 0–100 score of how well you actually understand the code your AI is shipping. Decays when you stop engaging. Climbs when you can answer for it.',
          style: TextStyle(
            color: AppColors.textSecondary,
            fontSize: 16,
            height: 1.6,
          ),
        ),
        const SizedBox(height: 24),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            _HeroButton(label: 'Install Rocky  →', primary: true, onTap: onInstall),
            _HeroButton(label: 'Learn more  ↓', primary: false, onTap: onLearnMore),
          ],
        ),
        const SizedBox(height: 20),
        Text(
          'Free, local, open source. Plugs into Claude Code, VS Code, your browser, and your git history.',
          style: TextStyle(
            color: AppColors.textMuted,
            fontSize: 13,
            height: 1.5,
          ),
        ),
      ],
    );
  }
}

class _HeroButton extends StatefulWidget {
  final String label;
  final bool primary;
  final VoidCallback onTap;
  const _HeroButton({required this.label, required this.primary, required this.onTap});
  @override
  State<_HeroButton> createState() => _HeroButtonState();
}

class _HeroButtonState extends State<_HeroButton> {
  bool _hovered = false;
  @override
  Widget build(BuildContext context) {
    final bg = widget.primary
        ? (_hovered ? AppColors.primaryDim : AppColors.primary)
        : (_hovered ? AppColors.sidebarHover : Colors.transparent);
    final fg = widget.primary ? Colors.black : AppColors.textPrimary;
    final border = widget.primary ? Colors.transparent : AppColors.divider;

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 13),
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: border, width: 1),
          ),
          child: Text(
            widget.label,
            style: TextStyle(
              color: fg,
              fontSize: 14,
              fontWeight: FontWeight.w600,
              letterSpacing: 0.2,
            ),
          ),
        ),
      ),
    );
  }
}

class _RockyIqDial extends StatefulWidget {
  final int target;
  const _RockyIqDial({required this.target});
  @override
  State<_RockyIqDial> createState() => _RockyIqDialState();
}

class _RockyIqDialState extends State<_RockyIqDial> with SingleTickerProviderStateMixin {
  late final AnimationController _ctrl;
  late final Animation<double> _anim;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(
      duration: const Duration(milliseconds: 1400),
      vsync: this,
    );
    _anim = CurvedAnimation(parent: _ctrl, curve: Curves.easeOutCubic);
    _ctrl.forward();
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  Color _colorFor(double v) {
    if (v >= 70) return TermColors.successGreen;
    if (v >= 40) return TermColors.fadingAmber;
    return TermColors.gapRed;
  }

  String _bucketFor(double v) {
    if (v >= 70) return 'KNOWN';
    if (v >= 40) return 'FADING';
    return 'GAP';
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (context, dark, __) => AnimatedBuilder(
        animation: _anim,
        builder: (context, _) {
          final v = _anim.value * widget.target;
          final color = _colorFor(v);
          // Pure black in light mode for max-contrast hero number;
          // theme-aware light tone in dark mode.
          final numberColor = dark ? AppColors.textPrimary : Colors.black;
          return SizedBox(
            width: 240,
            height: 240,
            child: Stack(
              alignment: Alignment.center,
              children: [
                CustomPaint(
                  size: const Size(240, 240),
                  painter: _DialPainter(
                    progress: v / 100,
                    color: color,
                    trackColor: AppColors.divider,
                  ),
                ),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      v.toInt().toString(),
                      style: TextStyle(
                        color: numberColor,
                        fontSize: 64,
                        fontWeight: FontWeight.w800,
                        height: 1,
                        letterSpacing: -2,
                      ),
                    ),
                  const SizedBox(height: 4),
                  Text(
                    '/ 100',
                    style: TextStyle(
                      color: AppColors.textMuted,
                      fontSize: 14,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                    decoration: BoxDecoration(
                      color: color.withValues(alpha: 0.15),
                      borderRadius: BorderRadius.circular(99),
                    ),
                    child: Text(
                      _bucketFor(v),
                      style: TextStyle(
                        color: color,
                        fontSize: 10,
                        fontWeight: FontWeight.w700,
                        letterSpacing: 1.5,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        );
        },
      ),
    );
  }
}

class _DialPainter extends CustomPainter {
  final double progress; // 0..1
  final Color color;
  final Color trackColor;
  _DialPainter({required this.progress, required this.color, required this.trackColor});

  static const double _twoPi = 6.2831853;
  static const double _topStart = -1.5707963; // -90°

  @override
  void paint(Canvas canvas, Size size) {
    const stroke = 14.0;
    final center = size.center(Offset.zero);
    final radius = (size.shortestSide - stroke) / 2;
    final rect = Rect.fromCircle(center: center, radius: radius);

    final track = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..color = trackColor
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(rect, 0, _twoPi, false, track);

    final arc = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round
      ..shader = SweepGradient(
        startAngle: 0,
        endAngle: _twoPi,
        colors: [color.withValues(alpha: 0.55), color],
      ).createShader(rect);

    final sweep = _twoPi * progress.clamp(0.0, 1.0);
    canvas.drawArc(rect, _topStart, sweep, false, arc);
  }

  @override
  bool shouldRepaint(covariant _DialPainter old) =>
      old.progress != progress || old.color != color || old.trackColor != trackColor;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
Future<void> _openUrl(String url) async {
  final parsedUri = Uri.parse(url);
  final absoluteUri =
      parsedUri.hasScheme ? parsedUri : Uri.base.resolveUri(parsedUri);
  if (await canLaunchUrl(absoluteUri)) {
    await launchUrl(absoluteUri, mode: LaunchMode.externalApplication);
  }
}
