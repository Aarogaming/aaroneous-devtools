// Isolated projection adapter. No I/O, persistence or authority decisions.
function createIndex(projection) {
  if (projection.schema_version !== 'topic-projection-experiment-v1') throw Error('unsupported projection');
  const topics = new Map();
  for (const topic of projection.topics) {
    if (!topic.id || topics.has(topic.id)) throw Error('duplicate or missing topic identity');
    topics.set(topic.id, topic);
  }
  for (const topic of topics.values()) {
    const seen = new Set(); let next = topic;
    while (next) {
      if (seen.has(next.id)) throw Error('parent cycle');
      seen.add(next.id);
      if (next.parent === null) break;
      next = topics.get(next.parent);
      if (!next) throw Error('missing parent');
    }
    for (const id of topic.children) if (topics.get(id)?.parent !== topic.id) throw Error('inconsistent child');
    if (topic.parent !== null && !topics.get(topic.parent).children.includes(topic.id)) throw Error('missing containment');
  }
  return {
    pathTo(id) {
      const path=[]; let topic=topics.get(id);
      if (!topic) throw Error('unknown topic');
      while (topic) { path.unshift(topic.id); topic=topics.get(topic.parent); }
      return path;
    },
    enter(path,id) {
      const current=topics.get(path.at(-1));
      if (!current?.children.includes(id)) throw Error('not a contained topic');
      const expected=this.pathTo(current.id);
      if(path.length!==expected.length || path.some((part,i)=>part!==expected[i])) throw Error('invalid topic path');
      return [...path,id];
    },
    related(id) {
      if (!topics.has(id)) throw Error('unknown topic');
      return projection.relations.filter(e=>e.from===id||e.to===id).map(e=>({
        assertionId:e.id,type:e.type,direction:e.from===id?'outgoing':'incoming',
        target:e.from===id?e.to:e.from,resolved:topics.has(e.from===id?e.to:e.from),
        status:e.status,containment:false
      }));
    }
  };
}
module.exports={createIndex};
