def walk_graph(graph, start):
    open = {start}
    seen = set()
    while open:
        node = open.pop()
        yield node
        seen.add(node)
        open.update(graph.get(node, set()) - seen)
