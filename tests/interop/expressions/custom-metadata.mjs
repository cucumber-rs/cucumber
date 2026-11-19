// Executed only inside the completed Phase32b BDD boundary.
import fs from 'node:fs';
import { CucumberExpression, RegularExpression, ParameterType, ParameterTypeRegistry } from '@cucumber/cucumber-expressions';
const { messages, mapping } = JSON.parse(fs.readFileSync(0, 'utf8'));
const registry = new ParameterTypeRegistry();
const descriptors = messages.filter(e => e.parameterType).map(e => e.parameterType);
for (const descriptor of descriptors) {
  if (!registry.lookupByTypeName(descriptor.name)) {
    registry.defineParameterType(new ParameterType(descriptor.name, descriptor.regularExpressions,
      String, value => value, descriptor.useForSnippets, descriptor.preferForRegularExpressionMatch));
  }
}
const definitions = new Map(messages.filter(e => e.stepDefinition).map(e => [e.stepDefinition.id, e.stepDefinition]));
const pickles = new Map(messages.filter(e => e.pickle).flatMap(e => e.pickle.steps.map(s => [s.id, s])));
const rawGroup = value => ({start: value.start, value: value.value, children: value.children?.map(rawGroup)});
const cases = messages.filter(e => e.testCase).flatMap(e => e.testCase.testSteps.filter(s => s.pickleStepId).map(step => {
  const text = pickles.get(step.pickleStepId).text;
  return { id: step.id, matches: step.stepDefinitionIds.map(id => {
    const definition = definitions.get(id);
    if (definition.pattern.type === 'CUCUMBER_EXPRESSION') {
      const expression = new CucumberExpression(definition.pattern.source, registry);
      return { id, arguments: expression.match(text).map(argument => ({
        group: rawGroup(argument.group), parameterTypeName: argument.parameterType.name,
      })) };
    }
    if (!mapping) return { id };
    const regexp = new RegExp(definition.pattern.source);
    const expression = new RegularExpression(regexp, registry);
    const args = expression.match(text);
    const native = new RegExp(regexp.source, 'd').exec(text);
    const groups = new Map();
    let index = 1;
    const observe = shape => {
      const current = index++;
      const value = {start: native.indices[current]?.[0], value: native[current], children: shape.children?.map(observe)};
      groups.set(current, value);
      return value;
    };
    const children = args.map(argument => observe(argument.group));
    groups.set(0, {start: native.indices[0][0], value: native[0], children: children.length ? children : undefined});
    return { id, arguments: mapping.map(capture => ({group: groups.get(capture)})) };
  }) };
}));
process.stdout.write(JSON.stringify({cases, descriptors}));
