// Runs only inside the completed Phase32a BDD acceptance boundary.
import fs from 'node:fs';
import { CucumberExpression, ParameterTypeRegistry } from '@cucumber/cucumber-expressions';
const { messages } = JSON.parse(fs.readFileSync(0, 'utf8'));
const registry = new ParameterTypeRegistry();
const definitions = new Map(messages.filter(e => e.stepDefinition).map(e => [e.stepDefinition.id, e.stepDefinition]));
const pickles = new Map(messages.filter(e => e.pickle).flatMap(e => e.pickle.steps.map(s => [s.id, s])));
const group = value => ({ start: value.start, value: value.value,
  children: value.children?.map(group) });
const cases = messages.filter(e => e.testCase).flatMap(e => e.testCase.testSteps.filter(s => s.pickleStepId).map(step => {
  const text = pickles.get(step.pickleStepId).text;
  return { id: step.id, matches: step.stepDefinitionIds.map(id => {
    const definition = definitions.get(id);
    if (definition.pattern.type !== 'CUCUMBER_EXPRESSION') return null;
    const expression = new CucumberExpression(definition.pattern.source, registry);
    const args = expression.match(text);
    // Independent modern engine evidence. Do not patch the pinned library or
    // reuse its regexp-match-indices polyfill when resolving these positions.
    const nativeMatch = new RegExp(expression.regexp.source, `${expression.regexp.flags}d`).exec(text);
    let nativeIndex = 1;
    const nativeGroup = shape => {
      const index = nativeIndex++;
      return { start: nativeMatch.indices[index]?.[0], value: nativeMatch[index],
        children: shape.children?.map(nativeGroup) };
    };
    return { id, arguments: args?.map(arg => ({group: group(arg.group), parameterTypeName: arg.parameterType.name})),
      nativeArguments: args?.map(arg => ({group: nativeGroup(arg.group), parameterTypeName: arg.parameterType.name})) };
  }) };
}));
const parameters = messages.filter(e => e.parameterType).map(e => {
  const parameter = registry.lookupByTypeName(e.parameterType.name);
  return { name: parameter.name, regularExpressions: parameter.regexpStrings,
    preferForRegularExpressionMatch: parameter.preferForRegexpMatch,
    useForSnippets: parameter.useForSnippets };
});
process.stdout.write(JSON.stringify({ cases, parameters }));
