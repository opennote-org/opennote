import 'package:flutter_test/flutter_test.dart';
import 'package:notes/state/app_state.dart';

void main() {
  test('switchDocumentTab cycles from active document', () {
    final appState = AppState();
    appState.openDocumentIds.addAll(['a', 'b', 'c']);

    appState.setActiveItem(ActiveItemType.document, 'b');
    appState.switchDocumentTab(1);
    expect(appState.activeItem.id, 'c');

    appState.switchDocumentTab(-1);
    expect(appState.activeItem.id, 'b');
  });

  test('switchDocumentTab uses last active document when active item differs', () {
    final appState = AppState();
    appState.openDocumentIds.addAll(['a', 'b', 'c']);

    appState.setActiveItem(ActiveItemType.document, 'b');
    appState.setActiveItem(ActiveItemType.collection, 'col1');

    appState.switchDocumentTab(1);
    expect(appState.activeItem.type, ActiveItemType.document);
    expect(appState.activeItem.id, 'c');
  });

  test('closeDocument activates next tab when closing active document', () {
    final appState = AppState();
    appState.openDocumentIds.addAll(['a', 'b', 'c']);
    appState.setActiveItem(ActiveItemType.document, 'b');

    appState.closeDocument('b');
    expect(appState.openDocumentIds, ['a', 'c']);
    expect(appState.activeItem.type, ActiveItemType.document);
    expect(appState.activeItem.id, 'c');
  });

  test('closeDocument activates previous tab when closing last tab', () {
    final appState = AppState();
    appState.openDocumentIds.addAll(['a', 'b', 'c']);
    appState.setActiveItem(ActiveItemType.document, 'c');

    appState.closeDocument('c');
    expect(appState.openDocumentIds, ['a', 'b']);
    expect(appState.activeItem.id, 'b');
  });

  test('closeDocument uses last active tab even if a collection is active', () {
    final appState = AppState();
    appState.openDocumentIds.addAll(['a', 'b', 'c']);
    appState.setActiveItem(ActiveItemType.document, 'b');
    appState.setActiveItem(ActiveItemType.collection, 'col1');

    appState.closeDocument('b');
    expect(appState.openDocumentIds, ['a', 'c']);
    expect(appState.activeItem.type, ActiveItemType.document);
    expect(appState.activeItem.id, 'c');
  });
}

