import 'package:flutter/material.dart';
import 'package:notes/state/app_state_scope.dart';

class NotificationCenterButton extends StatelessWidget {
  const NotificationCenterButton({super.key});

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final tasks = appState.tasks;
    final hasPending = tasks.any((t) => t.status == 'Pending' || t.status == 'InProgress');

    return PopupMenuButton<void>(
      tooltip: 'Notifications',
      icon: Stack(
        children: [
          const Icon(Icons.notifications),
          if (hasPending)
            Positioned(
              right: 0,
              top: 0,
              child: Container(
                padding: const EdgeInsets.all(2),
                decoration: const BoxDecoration(
                  color: Colors.red,
                  shape: BoxShape.circle,
                ),
                constraints: const BoxConstraints(
                  minWidth: 8,
                  minHeight: 8,
                ),
              ),
            ),
        ],
      ),
      offset: const Offset(0, 50),
      itemBuilder: (context) {
        if (tasks.isEmpty) {
          return [
            const PopupMenuItem(
              enabled: false,
              child: Text('No notifications'),
            ),
          ];
        }
        return tasks.map((task) {
          IconData icon;
          Color color;
          
          if (task.status == 'Success') {
            icon = Icons.check_circle;
            color = Colors.green;
          } else if (task.status == 'Failure') {
            icon = Icons.error;
            color = Colors.red;
          } else {
            icon = Icons.hourglass_empty;
            color = Colors.orange;
          }
          
          return PopupMenuItem(
            enabled: false, // Non-interactive items
            child: Row(
              children: [
                Icon(icon, color: color, size: 20),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        task.description,
                        style: const TextStyle(
                          fontWeight: FontWeight.normal,
                          color: Colors.white,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      if (task.message != null && task.message!.isNotEmpty)
                        Text(
                          task.message!,
                          style: const TextStyle(fontSize: 12, color: Colors.grey),
                          maxLines: 1000,
                          overflow: TextOverflow.ellipsis,
                        ),
                      Text(
                        task.status,
                        style: TextStyle(
                          fontSize: 10,
                          color: color,
                          fontStyle: FontStyle.italic,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          );
        }).toList();
      },
    );
  }
}
