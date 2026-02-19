import { ChevronLeft, ChevronRight, Clock, ListTodo, CalendarDays } from 'lucide-react';
import { useState } from 'react';

// ─── Calendar View ───────────────────────────────────────────────────────────

const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

function getDaysInMonth(year: number, month: number) {
    return new Date(year, month + 1, 0).getDate();
}

function getFirstDayOfMonth(year: number, month: number) {
    return new Date(year, month, 1).getDay();
}

export default function CalendarView() {
    const today = new Date();
    const [currentMonth, setCurrentMonth] = useState(today.getMonth());
    const [currentYear, setCurrentYear] = useState(today.getFullYear());

    const daysInMonth = getDaysInMonth(currentYear, currentMonth);
    const firstDay = getFirstDayOfMonth(currentYear, currentMonth);
    const isToday = (day: number) =>
        day === today.getDate() && currentMonth === today.getMonth() && currentYear === today.getFullYear();

    const prevMonth = () => {
        if (currentMonth === 0) {
            setCurrentMonth(11);
            setCurrentYear(y => y - 1);
        } else {
            setCurrentMonth(m => m - 1);
        }
    };

    const nextMonth = () => {
        if (currentMonth === 11) {
            setCurrentMonth(0);
            setCurrentYear(y => y + 1);
        } else {
            setCurrentMonth(m => m + 1);
        }
    };

    const goToToday = () => {
        setCurrentMonth(today.getMonth());
        setCurrentYear(today.getFullYear());
    };

    // Build the grid cells
    const cells: (number | null)[] = [];
    for (let i = 0; i < firstDay; i++) cells.push(null);
    for (let d = 1; d <= daysInMonth; d++) cells.push(d);

    return (
        <div className="flex h-full overflow-hidden">
            {/* Calendar Sidebar (Agenda) */}
            <div className="w-64 bg-zinc-900/60 flex flex-col border-r border-zinc-800/30 shrink-0">
                <div className="h-12 px-4 flex items-center gap-2 border-b border-zinc-800/30 shrink-0">
                    <Clock size={16} className="text-emerald-400" />
                    <span className="text-sm font-semibold text-zinc-200">Today's Agenda</span>
                </div>

                <div className="flex-1 overflow-y-auto p-3 space-y-2">
                    {/* Sample agenda items */}
                    <div className="p-3 rounded-lg bg-emerald-500/8 border border-emerald-500/15 space-y-1">
                        <div className="flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full bg-emerald-400" />
                            <span className="text-xs text-emerald-400 font-medium">9:00 AM</span>
                        </div>
                        <span className="text-sm text-zinc-300 block pl-4">Team Standup</span>
                    </div>
                    <div className="p-3 rounded-lg bg-blue-500/8 border border-blue-500/15 space-y-1">
                        <div className="flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full bg-blue-400" />
                            <span className="text-xs text-blue-400 font-medium">2:00 PM</span>
                        </div>
                        <span className="text-sm text-zinc-300 block pl-4">Design Review</span>
                    </div>
                    <div className="p-3 rounded-lg bg-amber-500/8 border border-amber-500/15 space-y-1">
                        <div className="flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full bg-amber-400" />
                            <span className="text-xs text-amber-400 font-medium">5:30 PM</span>
                        </div>
                        <span className="text-sm text-zinc-300 block pl-4">Gym Session</span>
                    </div>
                </div>

                {/* Mini To-Do */}
                <div className="border-t border-zinc-800/30 p-3 shrink-0">
                    <div className="flex items-center gap-2 mb-2">
                        <ListTodo size={14} className="text-zinc-500" />
                        <span className="text-xs font-semibold uppercase tracking-wider text-zinc-500">Tasks</span>
                    </div>
                    <div className="space-y-1.5">
                        {['Finish onboarding flow', 'Review PRs', 'Update docs'].map((task, i) => (
                            <label key={i} className="flex items-center gap-2 text-sm text-zinc-400 cursor-pointer group">
                                <div className="w-4 h-4 rounded border border-zinc-700 group-hover:border-emerald-500/50 transition-colors shrink-0" />
                                <span className="truncate">{task}</span>
                            </label>
                        ))}
                    </div>
                </div>
            </div>

            {/* Main Calendar Grid */}
            <div className="flex-1 flex flex-col bg-zinc-950/50 overflow-hidden">
                {/* Header */}
                <div className="h-14 px-6 flex items-center justify-between border-b border-zinc-800/30 shrink-0">
                    <div className="flex items-center gap-3">
                        <h2 className="text-lg font-bold text-zinc-100">
                            {MONTHS[currentMonth]} <span className="text-zinc-500 font-normal">{currentYear}</span>
                        </h2>
                        <button
                            onClick={goToToday}
                            className="text-xs font-medium text-emerald-400 hover:text-emerald-300 px-2 py-0.5 rounded-md hover:bg-emerald-500/10 transition-colors"
                        >
                            Today
                        </button>
                    </div>
                    <div className="flex items-center gap-1">
                        <button
                            onClick={prevMonth}
                            className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors"
                        >
                            <ChevronLeft size={16} />
                        </button>
                        <button
                            onClick={nextMonth}
                            className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors"
                        >
                            <ChevronRight size={16} />
                        </button>
                    </div>
                </div>

                {/* Day headers */}
                <div className="grid grid-cols-7 border-b border-zinc-800/30 shrink-0">
                    {DAYS.map(day => (
                        <div key={day} className="py-2 text-center text-xs font-semibold text-zinc-600 uppercase tracking-wider">
                            {day}
                        </div>
                    ))}
                </div>

                {/* Calendar grid */}
                <div className="flex-1 grid grid-cols-7 auto-rows-fr">
                    {cells.map((day, i) => (
                        <div
                            key={i}
                            className={`
                                border-b border-r border-zinc-800/20 p-1.5 min-h-0
                                ${day ? 'hover:bg-zinc-800/20 cursor-pointer transition-colors' : ''}
                            `}
                        >
                            {day && (
                                <div className={`
                                    w-7 h-7 rounded-full flex items-center justify-center text-xs font-medium
                                    ${isToday(day)
                                        ? 'bg-emerald-500 text-white shadow-lg shadow-emerald-500/30'
                                        : 'text-zinc-400 hover:text-zinc-200'
                                    }
                                `}>
                                    {day}
                                </div>
                            )}
                        </div>
                    ))}
                </div>

                {/* Coming Soon overlay */}
                <div className="absolute bottom-6 right-6 flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-xs font-medium text-emerald-400 pointer-events-none">
                    <CalendarDays size={12} />
                    <span>Calendar features coming soon</span>
                </div>
            </div>
        </div>
    );
}
