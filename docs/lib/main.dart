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
  const DocSection('Obsidian', Icons.hub, kObsidian),
  const DocSection('Sync & Backup', Icons.sync, kSync),
  const DocSection('Demo Usecase', Icons.timeline, kWalkthrough),
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
              child: SectionContent(
                key: ValueKey(_selected),
                markdown: kSections[_selected].markdown,
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

List<_MdPart> _parseParts(String markdown) {
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
      } else if (part is _CodePart && part.language == 'graphlink') {
        final segs = part.code.trim().split('|');
        widgets.add(Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: _GraphLink(
            linkPath: segs.isNotEmpty ? segs[0].trim() : '',
            label:    segs.length > 1 ? segs[1].trim() : 'Open interactive graph',
          ),
        ));
        i++;
      } else if (part is _CodePart && part.language == 'imagelink') {
        final segs = part.code.trim().split('|');
        widgets.add(Padding(
          padding: const EdgeInsets.only(bottom: 20),
          child: _GraphPreview(
            imagePath: segs.isNotEmpty ? segs[0].trim() : '',
            linkPath:  segs.length > 1 ? segs[1].trim() : '',
            caption:   segs.length > 2 ? segs[2].trim() : 'Open interactive graph',
          ),
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
class TerminalBlock extends StatefulWidget {
  final String code;
  final String language;
  /// Output to reveal instantly after the input finishes typing.
  final String? outputCode;
  const TerminalBlock({super.key, required this.code, required this.language, this.outputCode});

  @override
  State<TerminalBlock> createState() => _TerminalBlockState();
}

class _TerminalBlockState extends State<TerminalBlock>
    with TickerProviderStateMixin {
  late final AnimationController _typewriter;
  late final AnimationController _cursor;
  late final Animation<int> _charCount;
  bool _animationStarted = false;
  ScrollPosition? _scrollPos;

  late final String _inputSection;
  late final String _outputSection;

  @override
  void initState() {
    super.initState();

    // Split at the last shell-prompt line: everything up to and including it
    // is typed character-by-character; everything after appears all at once.
    final promptRe = RegExp(r'^[\w~/.]*\s*\$\s');
    final lines = widget.code.split('\n');
    int lastPrompt = -1;
    for (int i = 0; i < lines.length; i++) {
      if (promptRe.hasMatch(lines[i])) lastPrompt = i;
    }
    if (lastPrompt < 0 || lastPrompt == lines.length - 1) {
      _inputSection = widget.code;
      _outputSection = widget.outputCode ?? '';
    } else {
      _inputSection = lines.sublist(0, lastPrompt + 1).join('\n');
      final rest = lines.sublist(lastPrompt + 1).join('\n');
      _outputSection =
          widget.outputCode != null ? '$rest\n${widget.outputCode}' : rest;
    }

    final totalChars = _inputSection.length;
    // Scale duration: ~18ms per char, clamped between 600ms and 2800ms
    final ms = (totalChars * 18).clamp(600, 2800);
    _typewriter = AnimationController(
      vsync: this,
      duration: Duration(milliseconds: ms),
    );
    _charCount = IntTween(begin: 0, end: totalChars)
        .animate(CurvedAnimation(parent: _typewriter, curve: Curves.linear));

    _cursor = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 530),
    )..repeat(reverse: true);

    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final scrollable = Scrollable.maybeOf(context);
      _scrollPos = scrollable?.position;
      _scrollPos?.addListener(_checkVisibility);
      _checkVisibility();
    });
  }

  @override
  void dispose() {
    _scrollPos?.removeListener(_checkVisibility);
    _typewriter.dispose();
    _cursor.dispose();
    super.dispose();
  }

  void _checkVisibility() {
    if (_animationStarted || !mounted) return;
    final box = context.findRenderObject() as RenderBox?;
    if (box == null || !box.attached) return;
    final pos = box.localToGlobal(Offset.zero);
    final screenH = MediaQuery.sizeOf(context).height;
    if (pos.dy < screenH + 80 && pos.dy + box.size.height > -80) {
      _animationStarted = true;
      _typewriter.forward();
    }
  }

  String get _title {
    final lang = widget.language;
    if (lang.isEmpty) return 'output';
    return lang;
  }

  @override
  Widget build(BuildContext context) {
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
          _buildBody(),
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

  Widget _buildBody() {
    return AnimatedBuilder(
      animation: _charCount,
      builder: (context, _) {
        final visible = _inputSection.substring(0, _charCount.value);
        final lines = visible.split('\n');
        final isFinished = _charCount.value == _inputSection.length;

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
                for (final line in lines) _buildLine(line),
                // Cursor sits right after the typed input while still animating
                if (!isFinished)
                  AnimatedBuilder(
                    animation: _cursor,
                    builder: (_, __) => Text(
                      '█',
                      style: const TextStyle(
                        color: TermColors.cursor,
                        fontFamily: 'monospace',
                        fontSize: 14,
                        height: 1.0,
                      ),
                    ),
                  ),
                // Output revealed all at once when input finishes typing
                if (isFinished && _outputSection.isNotEmpty)
                  ...[
                    const SizedBox(height: 2),
                    ..._outputSection.split('\n').map(_buildLine),
                  ],
                // Blinking cursor at the bottom
                if (isFinished)
                  AnimatedBuilder(
                    animation: _cursor,
                    builder: (_, __) => Text(
                      '█',
                      style: TextStyle(
                        color: _cursor.value > 0.5
                            ? TermColors.cursor
                            : Colors.transparent,
                        fontFamily: 'monospace',
                        fontSize: 14,
                        height: 1.0,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        );
      },
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

  /// Style a single terminal line with semantic colours matching the Rocky CLI.
  TextSpan _styleLine(String line) {
    final trimmed = line.trimLeft();

    // Shell prompt:  ~/path $ command
    final promptMatch = RegExp(r'^([\w~/.]*\s*\$\s+)(.*)').firstMatch(line);
    if (promptMatch != null) {
      return TextSpan(children: [
        TextSpan(text: promptMatch.group(1),
            style: const TextStyle(color: TermColors.prompt, fontWeight: FontWeight.w600)),
        TextSpan(text: promptMatch.group(2),
            style: const TextStyle(color: TermColors.cmdText)),
      ]);
    }

    // ♫  Rocky personality / banner lines  →  Rocky brand amber
    if (trimmed.startsWith('♫')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.rockyBrand, fontWeight: FontWeight.w500));
    }

    // ✓  success
    if (trimmed.startsWith('✓')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.successGreen));
    }

    // ✗  error
    if (trimmed.startsWith('✗')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.gapRed));
    }

    // ~  fading / warning
    if (trimmed.startsWith('~')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.fadingAmber));
    }

    // Q1. / Q2. / Q3. …  Rocky is asking a question  →  Rocky feedback cyan
    if (RegExp(r'^Q\d+\.').hasMatch(trimmed)) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.rockyFeedback));
    }

    // Rocky: New topic …  Rocky is introducing something  →  Rocky brand amber
    if (trimmed.startsWith('Rocky:')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.rockyBrand, fontWeight: FontWeight.w500));
    }

    // > …  user answer  →  bright white, slightly italic
    if (trimmed.startsWith('>') && !trimmed.startsWith('> ') == false || trimmed.startsWith('> ')) {
      if (trimmed.startsWith('>')) {
        return TextSpan(text: line,
            style: const TextStyle(color: TermColors.userAnswer, fontStyle: FontStyle.italic));
      }
    }

    // [e] / [?] hint line  →  very muted
    if (trimmed.startsWith('[')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.hintMuted));
    }

    // Evaluating… / Fetching explanation… / Analyzing…  →  muted
    if (trimmed.startsWith('Evaluating') ||
        trimmed.startsWith('Fetching') ||
        trimmed.startsWith('Analyzing') ||
        trimmed.startsWith('Saved to PKG')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.hintMuted));
    }

    // # comment
    if (trimmed.startsWith('#')) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.commentTxt));
    }

    // Rocky's explanation / feedback body text (indented, after a ♫ or Q line)
    // — lines that are part of Rocky's voice: use feedback cyan
    if (line.startsWith('   ') && trimmed.isNotEmpty &&
        !trimmed.startsWith('Task:') &&
        !trimmed.startsWith('Total') && !trimmed.startsWith('Known') &&
        !trimmed.startsWith('Fading') && !trimmed.startsWith('Gaps') &&
        !trimmed.startsWith('Quiz') && !trimmed.startsWith('Edges') &&
        !trimmed.startsWith('Run ') && !trimmed.startsWith('Topic') &&
        !RegExp(r'^[A-Z][a-z].*\s+\|').hasMatch(trimmed) &&   // table rows
        !RegExp(r'^─').hasMatch(trimmed)) {
      return TextSpan(text: line,
          style: const TextStyle(color: TermColors.rockyFeedback));
    }

    return TextSpan(text: line,
        style: const TextStyle(color: TermColors.outputTxt));
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
// Graph link — inline "open in new tab" styled link to a live graph
// ---------------------------------------------------------------------------
class _GraphLink extends StatelessWidget {
  final String linkPath;
  final String label;
  const _GraphLink({required this.linkPath, required this.label});

  @override
  Widget build(BuildContext context) {
    final url = Uri.base.resolve(linkPath).toString();
    return GestureDetector(
      onTap: () => _openUrl(url),
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              label,
              style: TextStyle(
                color: AppColors.secondary,
                fontSize: 14,
                decoration: TextDecoration.underline,
                decorationColor: AppColors.secondary,
              ),
            ),
            const SizedBox(width: 4),
            Icon(Icons.open_in_new, size: 13, color: AppColors.secondary),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Graph preview — screenshot with link to live graph
// ---------------------------------------------------------------------------
class _GraphPreview extends StatelessWidget {
  final String imagePath;
  final String linkPath;
  final String caption;
  const _GraphPreview({required this.imagePath, required this.linkPath, required this.caption});

  @override
  Widget build(BuildContext context) {
    final imgUrl  = Uri.base.resolve(imagePath).toString();
    final linkUrl = Uri.base.resolve(linkPath).toString();

    return ValueListenableBuilder<bool>(
      valueListenable: _isDark,
      builder: (_, dark, __) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          GestureDetector(
            onTap: () => _openUrl(linkUrl),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                constraints: const BoxConstraints(maxHeight: 340),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(10),
                  border: Border.all(color: AppColors.divider, width: 1),
                ),
                clipBehavior: Clip.antiAlias,
                child: Image.network(
                  imgUrl,
                  fit: BoxFit.cover,
                  alignment: Alignment.topCenter,
                  errorBuilder: (_, __, ___) => Container(
                    height: 160,
                    color: AppColors.codeBg,
                    child: Center(
                      child: Text('Graph preview unavailable',
                          style: TextStyle(color: AppColors.textMuted, fontSize: 13)),
                    ),
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(height: 10),
          GestureDetector(
            onTap: () => _openUrl(linkUrl),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    caption,
                    style: TextStyle(
                      color: AppColors.secondary,
                      fontSize: 13,
                      decoration: TextDecoration.underline,
                      decorationColor: AppColors.secondary,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Icon(Icons.open_in_new, size: 12, color: AppColors.secondary),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
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
