# Food

Food is a fast, dependable personal nutrition and activity diary.

Its main goal is to require as little attention as possible. Logging should take
a few keys or taps, then get out of the way.

It lets a person:

- find, favourite, and record food quickly
- group food into meals and reuse meals
- create personal foods and recipes
- see energy and nutrient totals
- log water, workouts, steps, and body weight
- set dated calorie, nutrient, water, and weight goals
- account for normal daily burn and workout energy without counting either twice
- review weight, nutrition, and activity trends
- correct mistakes without changing unrelated history

The app opens on today, keeps safe defaults, supports keyboard use, and returns
to the next useful action after saving. Common actions do not need extra
confirmation.

The app stays quiet and focused. It shows useful results and errors, not routine
status messages.

The food catalog focuses on Finland. Fineli supplies common foods, with product
data where useful.

Food records facts and calculates estimates. It does not diagnose, prescribe, or
judge.

## Domain

### Food

A food has names and nutrition values for a fixed basis:

- 100 g
- 100 ml
- 1 count

The app does not guess conversions between mass, volume, and count.

A personal food follows the same rules as a catalog food.

Nutrition uses a controlled nutrient list. A missing nutrient value is unknown,
not zero. Imported sources keep all mapped nutrients, not only energy and
macros.

A person may save an exact shortcut for a food, such as `slice = 34 g`. A
shortcut must use the food's unit and only fills an amount. Imported serving
descriptions never become shortcuts and the app does not guess conversions.

Foods may be marked as favourites. Recent foods are derived from food entries.
Both are shown before a full search when useful.

### Food entry

A food entry records:

- the food
- the amount and matching unit
- when it was eaten
- a copy of the food and its nutrition
- the calculated consumed nutrients

Changing a food does not rewrite old entries.

### Meal

A meal groups food entries and may have a name. The name picker suggests
Breakfast, Lunch, Dinner, and Snack but accepts any text. A meal can also start
without a name.

The meal link is optional. A food entry stays valid and visible when its meal is
missing. A meal is created with its first food, so empty meals are not stored.

When logging food, the app automatically selects the most recent meal whose
latest entry is no more than two hours old. The selected meal stays visible. A
person may explicitly continue the latest meal even after two hours, but cannot
continue it if another meal was logged after it; repeating it starts a new meal.
The app does not guess the meal name from the time.

### Recipe

A recipe combines foods and their amounts. Its nutrition is calculated from its
ingredients. It can be logged by mass or count.

A logged recipe keeps its food and nutrition values even when the recipe is
changed.

### Water

A water entry records an amount in millilitres and when it was drunk. Logging
uses an interactive glass and bottle. Selecting a vessel restores its amount;
dragging the water level or using the keyboard adjusts it in 10 ml steps. The
glass starts at 250 ml and the bottle at 1,000 ml.

Drinks with energy or nutrients remain food entries. The app does not guess how
much other drinks should count as water.

### Workout

A workout records:

- the activity
- its start time
- its duration
- manually entered energy burned

The saved energy value belongs to that workout. Changing a workout type does not
rewrite old workouts.

A workout says what its energy estimate includes:

- **Active calories only:** add the full estimate to normal daily burn.
- **Includes resting calories:** subtract resting energy for the workout's
  duration before adding it.

The form uses a clear **Includes resting calories** toggle and explains why it
matters.

### Steps and activity data

Activity data may contain steps, resting energy, active energy, and workout
energy. Each value records its time and source.

The app must not count the same activity twice. A total from a wearable replaces
estimates already included in that total.

### Body weight

A body-weight entry records a measurement in kilograms and when it was taken.
Each measurement stays in the history.

The weight trend is calculated from measurements. It is not a separate record.

### Profile

The profile contains:

- locale
- timezone
- display units
- height
- age
- the inputs required by the chosen daily burn formula

Changing the profile does not rewrite recorded food, workouts, weight, or goals.

### Goals

Daily burn, food adjustment, water, nutrient, and weight goals start on a given
date. A day uses the latest goal profile that starts on or before that date. No
goals have default values.

Protein, carbohydrate, fat, and fibre totals are always shown. Other nutrient
totals are shown when the person sets a goal for them.

Normal daily burn covers rest and ordinary movement but excludes separately
logged workouts. It may be entered directly or estimated from the person's
profile. The food goal is daily burn plus a signed deficit or surplus. Logged
workout energy may increase it later.

### Day

A day follows the person's timezone.

The app calculates daily food, water, activity, energy, and nutrient totals from
the entries in that day. Calculated totals and trends are not separate records.

### Amount rules

- Amounts must be positive.
- Mass uses `g`.
- Volume uses `ml`.
- Counted items use `count`.
- Fractional counts are allowed.
- A food entry unit must match the food's nutrition basis.

### Feature set

- food search, favourites, recent foods, and recording
- meals and meal reuse
- personal foods and recipes
- water logging
- workouts and reusable workout types
- steps and activity data
- body-weight history and trends
- dated goals and day-specific overrides
- normal daily burn and workout energy
- daily totals and progress charts
- profile and unit settings
- editing and deleting entries

### Outside scope

- package tracking
- density conversion
- pantry, stock, or inventory tracking
- medical advice

## Main flows

### First sign in

Choose a locale and timezone before using the rest of the app. The locale
controls how values are shown. The timezone decides which timestamps belong to
today and other calendar days.

### Log food

1. Start typing a food name.
2. Select the first result with Enter or tap another result.
3. Enter the amount.
4. Optionally name a new meal. The picker suggests common names but accepts any
   text.
5. Save.

The search receives focus again so another food can be logged at once.

### Repeat a meal

1. Search for a meal or use its copy control.
2. Uncheck foods that are not wanted and adjust amounts.
3. Copy the selected foods together.

Copying always starts a new meal with the source meal's name and current food
definitions. Unavailable foods cannot be selected.

### Add water

1. Choose the glass or bottle.
2. Drag the water level, or use the arrow keys, to adjust the amount.
3. Add the shown amount without leaving the day.

### Log a workout

1. Choose a saved workout type.
2. Enter or adjust the duration.
3. Enter energy burned.
4. Save.

The form keeps safe values from the chosen workout type.

### Weigh in

Enter the weight and save. The current time is used unless changed.

### Review a day

Opening the app shows today. Food, water, activity, goals, and totals are
visible without another action. Previous days are one step away.

### Change a goal

Choose the goal, value, and start date. The change applies from that date and
does not alter earlier days.

### Interaction rules

- Every link and form works without JavaScript.
- htmx may make working navigation and forms faster, but does not replace their
  normal HTTP behavior.
- Browser-only enhancements also initialize after htmx replaces page content.
- Optimistic updates are used only when failure has a clear rollback.
- Common logging works with a keyboard or one-handed taps.
- Ask only for values needed to save a valid entry.
- Reuse safe values instead of asking again.
- Do not guess values that can change the meaning of an entry.
- Do not show routine success messages.
- Keep corrections easy.
