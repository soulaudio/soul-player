/**
 * Theme Contrast Validation Report
 * Validates WCAG 2.1 AA compliance for all theme color combinations
 */

function getRelativeLuminance(rgb) {
  const [r, g, b] = rgb
  const sRGB = [r, g, b].map(channel => {
    const val = channel / 255
    return val <= 0.03928 ? val / 12.92 : Math.pow((val + 0.055) / 1.055, 2.4)
  })
  return 0.2126 * sRGB[0] + 0.7152 * sRGB[1] + 0.0722 * sRGB[2]
}

function getContrastRatio(color1, color2) {
  const lum1 = getRelativeLuminance(color1)
  const lum2 = getRelativeLuminance(color2)
  const lighter = Math.max(lum1, lum2)
  const darker = Math.min(lum1, lum2)
  return (lighter + 0.05) / (darker + 0.05)
}

function checkWCAG(ratio, isLargeText) {
  const passAA = isLargeText ? ratio >= 3.0 : ratio >= 4.5
  const passAAA = isLargeText ? ratio >= 4.5 : ratio >= 7.0
  return { passAA, passAAA, ratio: ratio.toFixed(2) }
}

// Theme configurations - COMPREHENSIVE TEST SUITE
const themes = {
  dark: {
    name: 'Dark Theme',
    background: [0, 0, 0],
    elements: {
      // Hero text elements
      'Soul Player heading (h2)': { color: [244, 244, 245], isLarge: true }, // zinc-100
      'Soul Audio subtitle': { color: [161, 161, 170], isLarge: false }, // zinc-400
      'Main tagline text': { color: [212, 212, 216], isLarge: false }, // zinc-300
      'Description paragraph': { color: [161, 161, 170], isLarge: false }, // zinc-400
      'Theme picker label': { color: [161, 161, 170], isLarge: false }, // zinc-400
      'Badge text (No subs/ads)': { color: [161, 161, 170], isLarge: false }, // zinc-400

      // Interactive elements
      'Theme button (inactive)': { color: [228, 228, 231], bg: [39, 39, 42], isLarge: false }, // zinc-200 on zinc-800
      'Theme button (active)': { color: [255, 255, 255], bg: [109, 40, 217], isLarge: false }, // white on violet-700

      // Gradient text - FURTHER DARKENED (was violet-400 to violet-500, now violet-600 to violet-650 custom)
      'Gradient text (lightest)': { color: [124, 58, 237], isLarge: true }, // violet-600 (darker for balance)
      'Gradient text (darkest)': { color: [117, 49, 227], isLarge: true }, // violet-650 (custom - between 600 and 700)
    },
  },
  light: {
    name: 'Light Theme',
    background: [250, 249, 255],
    elements: {
      // Hero text elements
      'Soul Player heading (h2)': { color: [24, 24, 27], isLarge: true }, // zinc-900
      'Soul Audio subtitle': { color: [63, 63, 70], isLarge: false }, // zinc-700
      'Main tagline text': { color: [39, 39, 42], isLarge: false }, // zinc-800
      'Description paragraph': { color: [82, 82, 91], isLarge: false }, // zinc-600
      'Theme picker label': { color: [82, 82, 91], isLarge: false }, // zinc-600
      'Badge text (No subs/ads)': { color: [113, 113, 122], isLarge: false }, // zinc-500

      // Interactive elements
      'Theme button (inactive)': { color: [63, 63, 70], bg: [255, 255, 255], isLarge: false }, // zinc-700 on white
      'Theme button (active)': { color: [255, 255, 255], bg: [109, 40, 217], isLarge: false }, // white on violet-700

      // Gradient text (FIXED - much darker violets)
      'Gradient text (lightest)': { color: [109, 40, 217], isLarge: true }, // violet-700
      'Gradient text (darkest)': { color: [88, 28, 135], isLarge: true }, // violet-900
    },
  },
  ocean: {
    name: 'Ocean Theme (Light)',
    background: [224, 242, 254], // sky-100 - LIGHT theme now!
    elements: {
      // Hero text elements - SAME AS LIGHT THEME
      'Soul Player heading (h2)': { color: [24, 24, 27], isLarge: true }, // zinc-900
      'Soul Audio subtitle': { color: [63, 63, 70], isLarge: false }, // zinc-700
      'Main tagline text': { color: [39, 39, 42], isLarge: false }, // zinc-800
      'Description paragraph': { color: [82, 82, 91], isLarge: false }, // zinc-600
      'Theme picker label': { color: [82, 82, 91], isLarge: false }, // zinc-600
      'Badge text (No subs/ads)': { color: [82, 82, 91], isLarge: false }, // zinc-600 (better contrast on sky-100)

      // Interactive elements - SAME AS LIGHT THEME
      'Theme button (inactive)': { color: [63, 63, 70], bg: [255, 255, 255], isLarge: false }, // zinc-700 on white
      'Theme button (active)': { color: [255, 255, 255], bg: [109, 40, 217], isLarge: false }, // white on violet-700

      // Gradient text - SAME AS LIGHT THEME
      'Gradient text (lightest)': { color: [109, 40, 217], isLarge: true }, // violet-700
      'Gradient text (darkest)': { color: [88, 28, 135], isLarge: true }, // violet-900
    },
  },
}

console.log('\\n' + '='.repeat(80))
console.log('  WCAG 2.1 CONTRAST VALIDATION REPORT')
console.log('  Soul Player Marketing Site - Theme Analysis')
console.log('='.repeat(80) + '\\n')

let totalChecks = 0
let passedAA = 0
let passedAAA = 0
let failures = []

for (const [themeId, theme] of Object.entries(themes)) {
  console.log(`\\n📋 ${theme.name}`)
  console.log('─'.repeat(80))
  console.log(`Background: rgb(${theme.background.join(', ')})\\n`)

  for (const [element, config] of Object.entries(theme.elements)) {
    totalChecks++
    // Use custom background if specified, otherwise use theme background
    const bgColor = config.bg || theme.background
    const ratio = getContrastRatio(config.color, bgColor)
    const result = checkWCAG(ratio, config.isLarge)

    const icon = result.passAA ? '✅' : '❌'
    const standard = config.isLarge ? 'Large Text (3:1 AA, 4.5:1 AAA)' : 'Normal Text (4.5:1 AA, 7:1 AAA)'

    console.log(`  ${icon} ${element}`)
    console.log(`     Color: rgb(${config.color.join(', ')})`)
    if (config.bg) {
      console.log(`     Background: rgb(${bgColor.join(', ')})`)
    }
    console.log(`     Contrast Ratio: ${result.ratio}:1`)
    console.log(`     Standard: ${standard}`)
    console.log(`     WCAG AA: ${result.passAA ? '✓ PASS' : '✗ FAIL'} | WCAG AAA: ${result.passAAA ? '✓ PASS' : '✗ FAIL'}`)
    console.log()

    if (result.passAA) passedAA++
    if (result.passAAA) passedAAA++
    if (!result.passAA) {
      failures.push({
        theme: theme.name,
        element,
        ratio: result.ratio,
        required: config.isLarge ? '3.0' : '4.5',
      })
    }
  }
}

console.log('\\n' + '='.repeat(80))
console.log('  SUMMARY')
console.log('='.repeat(80))
console.log(`Total Checks: ${totalChecks}`)
console.log(`WCAG AA Compliance: ${passedAA}/${totalChecks} (${((passedAA / totalChecks) * 100).toFixed(1)}%)`)
console.log(`WCAG AAA Compliance: ${passedAAA}/${totalChecks} (${((passedAAA / totalChecks) * 100).toFixed(1)}%)`)

if (failures.length > 0) {
  console.log(`\\n⚠️  ${failures.length} WCAG AA Failure(s):\\n`)
  failures.forEach((fail, i) => {
    console.log(`   ${i + 1}. ${fail.theme} - ${fail.element}`)
    console.log(`      Ratio: ${fail.ratio}:1 (Required: ${fail.required}:1)`)
  })
} else {
  console.log('\\n✨ All color combinations meet WCAG AA standards!')
}

console.log('\\n' + '='.repeat(80) + '\\n')
