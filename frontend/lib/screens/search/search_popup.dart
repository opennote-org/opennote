import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:notes/services/search.dart';
import 'package:notes/state/app_state_scope.dart';

class SearchPopup extends StatefulWidget {
  final String scopeId;
  final SearchScope scope;

  const SearchPopup({super.key, required this.scopeId, required this.scope});

  @override
  State<SearchPopup> createState() => _SearchPopupState();
}

class _SearchPopupState extends State<SearchPopup> {
  final TextEditingController _queryController = TextEditingController();
  List<DocumentChunkSearchResult> _results = [];
  bool _isLoading = false;
  bool _isKeywordSearch = false;
  bool _isInitialized = false;
  int _topN = 10;
  Timer? _debounce;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_isInitialized) {
      _initializeData();
      _isInitialized = true;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      elevation: 0,
      backgroundColor: Colors.transparent,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 600, maxHeight: 700),
        child: Container(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surface,
            borderRadius: BorderRadius.circular(16),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.1),
                blurRadius: 20,
                offset: const Offset(0, 10),
              ),
            ],
          ),
          child: Column(
            children: [
              _buildHeader(),
              const Divider(height: 1),
              Expanded(child: _buildResults()),
            ],
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _queryController.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
  }

  Widget _buildHeader() {
    return Padding(
      padding: const EdgeInsets.all(16.0),
      child: Column(
        children: [
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _queryController,
                  decoration: InputDecoration(
                    hintText: 'Search...',
                    prefixIcon: const Icon(Icons.search),
                    filled: true,
                    fillColor: Theme.of(
                      context,
                    ).colorScheme.surfaceContainerHighest.withOpacity(0.3),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                      borderSide: BorderSide.none,
                    ),
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 12,
                    ),
                  ),
                  textInputAction: TextInputAction.search,
                  onSubmitted: (_) => _search(),
                  onChanged: _onSearchChanged,
                  autofocus: true,
                ),
              ),
              const SizedBox(width: 12),
              IconButton.filledTonal(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              FilterChip(
                label: const Text('Semantic Search'),
                selected: !_isKeywordSearch,
                onSelected: (selected) {
                  if (selected && _isKeywordSearch) {
                    setState(() => _isKeywordSearch = false);
                    _search();
                  }
                },
                showCheckmark: false,
                avatar: !_isKeywordSearch
                    ? const Icon(Icons.auto_awesome, size: 16)
                    : null,
              ),
              const SizedBox(width: 8),
              FilterChip(
                label: const Text('Keyword Search'),
                selected: _isKeywordSearch,
                onSelected: (selected) {
                  if (selected && !_isKeywordSearch) {
                    setState(() => _isKeywordSearch = true);
                    _search();
                  }
                },
                showCheckmark: false,
                avatar: _isKeywordSearch
                    ? const Icon(Icons.text_fields, size: 16)
                    : null,
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildHighlightedText(String text, String query) {
    if (query.isEmpty || !_isKeywordSearch) {
      return Text(
        text,
        style: const TextStyle(fontWeight: FontWeight.w500),
        maxLines: 1000,
      );
    }

    final List<InlineSpan> spans = [];
    final String lowercaseText = text.toLowerCase();
    final String lowercaseQuery = query.toLowerCase();

    int start = 0;
    int indexOfHighlight = lowercaseText.indexOf(lowercaseQuery);

    while (indexOfHighlight != -1) {
      if (indexOfHighlight > start) {
        spans.add(TextSpan(text: text.substring(start, indexOfHighlight)));
      }

      spans.add(
        TextSpan(
          text: text.substring(
            indexOfHighlight,
            indexOfHighlight + query.length,
          ),
          style: TextStyle(
            backgroundColor: Theme.of(context).colorScheme.tertiaryContainer,
            color: Theme.of(context).colorScheme.onTertiaryContainer,
          ),
        ),
      );

      start = indexOfHighlight + query.length;
      indexOfHighlight = lowercaseText.indexOf(lowercaseQuery, start);
    }

    if (start < text.length) {
      spans.add(TextSpan(text: text.substring(start)));
    }

    return Text.rich(
      TextSpan(children: spans),
      style: const TextStyle(fontWeight: FontWeight.w500),
      maxLines: 1000,
    );
  }

  Widget _buildResults() {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_results.isEmpty && _queryController.text.isNotEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.search_off,
              size: 48,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: 16),
            Text(
              'No results found',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
          ],
        ),
      );
    }

    if (_results.isEmpty) {
      return Center(
        child: Text(
          'Type to search...',
          style: Theme.of(context).textTheme.bodyLarge?.copyWith(
            color: Theme.of(context).colorScheme.outline,
          ),
        ),
      );
    }

    return ListView.separated(
      padding: const EdgeInsets.symmetric(vertical: 8),
      itemCount: _results.length,
      separatorBuilder: (context, index) =>
          const Divider(height: 1, indent: 16, endIndent: 16),
      itemBuilder: (context, index) {
        final result = _results[index];
        return ListTile(
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 8,
          ),
          title: _buildHighlightedText(
            result.documentChunk.content.trim(),
            _queryController.text.trim(),
          ),
          subtitle: Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8.0,
              children: [
                Text(
                  'From ${result.documentTitle ?? '???'} > ${result.collectionTitle ?? '???'}',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.secondary,
                  ),
                ),
                Text(
                  'Score: ${(result.score * 100).toStringAsFixed(0)}%',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.secondary,
                  ),
                ),
              ],
            ),
          ),
          onTap: () async {
            final appState = AppStateScope.of(context);
            await appState.openDocument(
              result.documentChunk.documentMetadataId,
              collectionId: result.documentChunk.collectionMetadataId,
              highlightText: result.documentChunk.content,
              highlightChunkId: result.documentChunk.id,
            );
            if (!context.mounted) return;
            Navigator.pop(context);
          },
          hoverColor: Theme.of(
            context,
          ).colorScheme.surfaceContainerHighest.withOpacity(0.5),
        );
      },
    );
  }

  Future<void> _initializeData() async {
    final appState = AppStateScope.of(context);
    try {
      final configurations = await appState.users.getUserConfigurationsMap(
        Dio(),
        appState.username!,
      );

      if (!mounted) return;
      setState(() {
        if (configurations.containsKey('search') &&
            configurations['search'] is Map) {
          final searchConfig = configurations['search'] as Map<String, dynamic>;
          final defaultSearchMethod = searchConfig['default_search_method'];

          if (defaultSearchMethod == 'semantic') {
            _isKeywordSearch = false;
          } else if (defaultSearchMethod == 'keyword') {
            _isKeywordSearch = true;
          }

          if (searchConfig.containsKey('top_n')) {
            _topN = (searchConfig['top_n'] as num).toInt();
          }
        }
      });
    } catch (e) {
      // debugPrint('Failed to load user configurations: $e');
    }
  }

  void _onSearchChanged(String query) {
    if (_debounce?.isActive ?? false) _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 250), () {
      _search();
    });
  }

  Future<void> _search() async {
    final query = _queryController.text.trim();
    if (query.isEmpty) {
      setState(() => _results = []);
      return;
    }

    setState(() => _isLoading = true);
    try {
      final appState = AppStateScope.of(context);
      final searchService = SearchService();
      final results = _isKeywordSearch
          ? await searchService.keywordSearch(
              appState.dio,
              query: query,
              scope: widget.scope,
              scopeId: widget.scopeId,
              topN: _topN,
            )
          : await searchService.intelligentSearch(
              appState.dio,
              query: query,
              scope: widget.scope,
              scopeId: widget.scopeId,
              topN: _topN,
            );
      setState(() {
        _results = results;
      });
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Search failed: $e')));
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }
}
