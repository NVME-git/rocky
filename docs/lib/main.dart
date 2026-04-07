import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:url_launcher/url_launcher.dart';
import 'content.dart';

void main() {
  runApp(const RockyDocsApp());
}

// ---------------------------------------------------------------------------
// Color palette
// ---------------------------------------------------------------------------
class AppColors {
  static const background = Color(0xFF0A0E1A);
  static const surface = Color(0xFF111827);
  static const sidebarBg = Color(0xFF070B14);
  static const sidebarHover = Color(0xFF0F1629);
  static const primary = Color(0xFFF59E0B); // amber
  static const primaryDim = Color(0xFFD97706);
  static const secondary = Color(0xFF06B6D4); // cyan
  static const textPrimary = Color(0xFFF1F5F9);
  static const textSecondary = Color(0xFFCBD5E1);
  static const textMuted = Color(0xFF64748B);
  static const codeBg = Color(0xFF1E293B);
  static const codeBorder = Color(0xFF334155);
  static const divider = Color(0xFF1E293B);
  static const cardBg = Color(0xFF0F172A);
}

// ---------------------------------------------------------------------------
// App root
// ---------------------------------------------------------------------------
class RockyDocsApp extends StatelessWidget {
  const RockyDocsApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Rocky',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true).copyWith(
        scaffoldBackgroundColor: AppColors.background,
        colorScheme: const ColorScheme.dark(
          primary: AppColors.primary,
          secondary: AppColors.secondary,
          surface: AppColors.surface,
        ),
        dividerColor: AppColors.divider,
        textTheme: ThemeData.dark().textTheme.apply(
          bodyColor: AppColors.textPrimary,
          displayColor: AppColors.textPrimary,
          fontFamily: 'Roboto',
        ),
      ),
      home: const DocsShell(),
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
  const DocSection('Project Walkthrough', Icons.timeline, kWalkthrough),
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
    if (_isNarrow(context)) Navigator.of(context).pop(); // close drawer
  }

  bool _isNarrow(BuildContext context) =>
      MediaQuery.of(context).size.width < _breakpoint;

  @override
  Widget build(BuildContext context) {
    final narrow = _isNarrow(context);
    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: narrow
          ? AppBar(
              backgroundColor: AppColors.sidebarBg,
              title: const _LogoRow(compact: true),
              elevation: 0,
            )
          : null,
      drawer: narrow ? Drawer(child: _buildSidebar()) : null,
      body: Row(
        children: [
          if (!narrow) _buildSidebar(),
          Expanded(child: _buildContent()),
        ],
      ),
    );
  }

  Widget _buildSidebar() {
    return Container(
      width: _sidebarWidth,
      decoration: const BoxDecoration(
        color: AppColors.sidebarBg,
        border: Border(right: BorderSide(color: AppColors.divider, width: 1)),
      ),
      child: Column(
        children: [
          const SizedBox(height: 24),
          const _LogoRow(),
          const SizedBox(height: 8),
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 24),
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
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 24),
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
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 24),
            child: Divider(color: AppColors.divider, height: 1),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                const Icon(Icons.code, size: 14, color: AppColors.textMuted),
                const SizedBox(width: 8),
                GestureDetector(
                  onTap: () => _openUrl('https://github.com/NVME-git/rocky'),
                  child: const Text(
                    'GitHub',
                    style: TextStyle(color: AppColors.textMuted, fontSize: 12),
                  ),
                ),
                const Spacer(),
                const Text(
                  'MIT License',
                  style: TextStyle(color: AppColors.textMuted, fontSize: 11),
                ),
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
              child: StyledMarkdown(
                data: kSections[_selected].markdown,
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
              gradient: const LinearGradient(
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
          const Text(
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
            child: const Text(
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
              Text(
                widget.label,
                style: TextStyle(
                  color: fg,
                  fontSize: 14,
                  fontWeight:
                      widget.active ? FontWeight.w600 : FontWeight.w400,
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
// Styled Markdown renderer
// ---------------------------------------------------------------------------
class StyledMarkdown extends StatelessWidget {
  final String data;
  const StyledMarkdown({super.key, required this.data});

  @override
  Widget build(BuildContext context) {
    return MarkdownBody(
      data: data,
      selectable: true,
      onTapLink: (text, href, title) {
        if (href != null) _openUrl(href);
      },
      styleSheet: MarkdownStyleSheet(
        // Headings
        h1: const TextStyle(
          color: AppColors.textPrimary,
          fontSize: 32,
          fontWeight: FontWeight.w800,
          height: 1.3,
          letterSpacing: -0.5,
        ),
        h1Padding: const EdgeInsets.only(bottom: 16, top: 8),
        h2: const TextStyle(
          color: AppColors.textPrimary,
          fontSize: 24,
          fontWeight: FontWeight.w700,
          height: 1.4,
        ),
        h2Padding: const EdgeInsets.only(bottom: 12, top: 32),
        h3: const TextStyle(
          color: AppColors.textPrimary,
          fontSize: 18,
          fontWeight: FontWeight.w600,
          height: 1.4,
        ),
        h3Padding: const EdgeInsets.only(bottom: 8, top: 24),
        // Body
        p: const TextStyle(
          color: AppColors.textSecondary,
          fontSize: 15,
          height: 1.7,
        ),
        pPadding: const EdgeInsets.only(bottom: 12),
        // Strong / emphasis
        strong: const TextStyle(
          color: AppColors.textPrimary,
          fontWeight: FontWeight.w700,
        ),
        em: const TextStyle(
          color: AppColors.textSecondary,
          fontStyle: FontStyle.italic,
        ),
        // Links
        a: const TextStyle(
          color: AppColors.secondary,
          decoration: TextDecoration.underline,
          decorationColor: AppColors.secondary,
        ),
        // Code
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
        // Lists
        listBullet: const TextStyle(color: AppColors.primary, fontSize: 14),
        listBulletPadding: const EdgeInsets.only(right: 8),
        listIndent: 24,
        // Blockquote
        blockquote: const TextStyle(
          color: AppColors.textMuted,
          fontSize: 15,
          height: 1.7,
          fontStyle: FontStyle.italic,
        ),
        blockquoteDecoration: BoxDecoration(
          border: const Border(
            left: BorderSide(color: AppColors.primary, width: 3),
          ),
          color: AppColors.primary.withValues(alpha: 0.06),
          borderRadius: const BorderRadius.only(
            topRight: Radius.circular(4),
            bottomRight: Radius.circular(4),
          ),
        ),
        blockquotePadding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
        // Horizontal rule
        horizontalRuleDecoration: const BoxDecoration(
          border: Border(
            top: BorderSide(color: AppColors.divider, width: 1),
          ),
        ),
        // Tables
        tableHead: const TextStyle(
          color: AppColors.textPrimary,
          fontWeight: FontWeight.w600,
          fontSize: 14,
        ),
        tableBody: const TextStyle(
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
        tableCellsPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
      imageBuilder: (uri, title, alt) {
        final resolved = uri.hasScheme ? uri : Uri.base.resolveUri(uri);
        return Padding(
          padding: const EdgeInsets.symmetric(vertical: 12),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(8),
            child: Image.network(
              resolved.toString(),
              fit: BoxFit.contain,
              errorBuilder: (context, error, stackTrace) => Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: AppColors.codeBg,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: AppColors.codeBorder),
                ),
                child: Text(
                  alt ?? 'Image',
                  style: const TextStyle(color: AppColors.textMuted, fontSize: 13),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
Future<void> _openUrl(String url) async {
  final parsed = Uri.parse(url);
  final uri = parsed.hasScheme ? parsed : Uri.base.resolveUri(parsed);
  if (await canLaunchUrl(uri)) {
    await launchUrl(uri, mode: LaunchMode.externalApplication);
  }
}
