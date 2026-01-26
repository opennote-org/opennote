import 'dart:async';

import 'package:flutter/material.dart';
import 'package:notes/constants.dart';
import 'package:notes/state/app_state_scope.dart';

class SystemPanel extends StatefulWidget {
  const SystemPanel();

  @override
  State<SystemPanel> createState() => _SystemPanelState();
}

class _SystemPanelState extends State<SystemPanel> {
  bool _isLoading = false;
  String backendServiceVersionNumber = "N/a";

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _load();
    });
  }

  Future<void> _load() async {
    final appState = AppStateScope.of(context);
    if (appState.username == null) return;

    setState(() => _isLoading = true);
    backendServiceVersionNumber = await appState
        .getBackendServiceVersionNumber();
    if (mounted) {
      setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 24,
      children: [
        Text("System", style: Theme.of(context).textTheme.titleMedium),
        Text("Backend Version Number: $backendServiceVersionNumber"),
        Text("Frontend Version Number: $frontendVersion"),
      ],
    );
  }
}
