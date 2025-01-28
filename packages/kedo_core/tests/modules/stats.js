export const mean = (arr) => arr.reduce((a, b) => a + b, 0) / arr.length;

export const median = (arr) => {
  arr.sort((a, b) => a - b);
  const mid = Math.floor(arr.length / 2);
  return arr.length % 2 === 0 ? (arr[mid - 1] + arr[mid]) / 2 : arr[mid];
};
