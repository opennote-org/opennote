import 'package:flutter/material.dart';
import 'package:notes/screens/auth/login.dart';
import 'package:notes/screens/main_screen.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';

void main() async {
  final state = AppState();
  runApp(AppStateScope(notifier: state, child: const MyApp()));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Notes',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.deepPurple,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      home: const AuthWrapper(),
    );
  }
}

class AuthWrapper extends StatelessWidget {
  const AuthWrapper({super.key});

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    if (appState.username != null) {
      return const MainScreen();
    }
    return const LoginScreen();
  }
}
