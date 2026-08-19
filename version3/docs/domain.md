# Domain

## Food

A food has names and nutrition values for a fixed basis:

- 100 g
- 100 ml
- 1 count

The app does not guess conversions between mass, volume, and count.

A personal food follows the same rules as a catalog food.

Nutrition uses a controlled nutrient list. A missing nutrient value is unknown,
not zero. Imported sources keep all mapped nutrients, not only energy and macros.

A person may save an exact shortcut for a food, such as `slice = 34 g`. A
shortcut must use the food's unit and only fills an amount. Imported serving
descriptions never become shortcuts and the app does not guess conversions.

Foods may be marked as favourites. Recent foods are derived from food entries.
Both are shown before a full search when useful.

## Food entry

A food entry records:

- the food
- the amount and matching unit
- when it was eaten
- a copy of the food and its nutrition
- the calculated consumed nutrients

Changing a food does not rewrite old entries.

## Meal

A meal groups food entries. Its kind is breakfast, lunch, dinner, or snack.

The meal link is optional. A food entry stays valid and visible when its meal is
missing. Empty meals are hidden.

When logging food, the app automatically selects the most recent meal whose
latest entry is no more than two hours old. The selected meal stays visible and
the person can choose another meal or start a new one. The app does not guess
the meal kind from the time.

## Recipe

A recipe combines foods and their amounts. Its nutrition is calculated from its
ingredients. It can be logged by mass or count.

A logged recipe keeps its food and nutrition values even when the recipe is
changed.

## Water

A water entry records an amount in millilitres and when it was drunk. Logging
offers common amounts and a custom amount.

Drinks with energy or nutrients remain food entries. The app does not guess how
much other drinks should count as water.

## Workout

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

## Steps and activity data

Activity data may contain steps, resting energy, active energy, and workout
energy. Each value records its time and source.

The app must not count the same activity twice. A total from a wearable replaces
estimates already included in that total.

## Body weight

A body-weight entry records a measurement in kilograms and when it was taken.
Each measurement stays in the history.

The weight trend is calculated from measurements. It is not a separate record.

## Profile

The profile contains:

- timezone
- display units
- height
- age
- the inputs required by the chosen daily burn formula

Changing the profile does not rewrite recorded food, workouts, weight, or goals.

## Goals

Calorie, protein, carbohydrate, fat, fibre, water, and weight goals start on a
given date. A day uses the goals that apply to that date. A single day may have
an override.

Normal daily burn covers rest and ordinary movement but excludes separately
logged workouts. It may be entered directly or estimated from the person's
profile.

An energy goal may include a desired deficit or surplus and whether workout
energy changes the food target.

## Day

A day follows the person's timezone.

The app calculates daily food, water, activity, energy, and nutrient totals from
the entries in that day. Calculated totals and trends are not separate records.

## Amount rules

- Amounts must be positive.
- Mass uses `g`.
- Volume uses `ml`.
- Counted items use `count`.
- Fractional counts are allowed.
- A food entry unit must match the food's nutrition basis.

## Feature set

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

## Outside scope

- package tracking
- density conversion
- pantry, stock, or inventory tracking
- medical advice
