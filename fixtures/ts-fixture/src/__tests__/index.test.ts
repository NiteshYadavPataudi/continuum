import { add, greet } from '../index';

test('add', () => {
    expect(add(2, 2)).toBe(4);
});

test('greet', () => {
    expect(greet('World')).toBe('Hello, World!');
});
